use super::types::*;
use super::deserialize::deserialize_element;
use super::interner::Interner;

use std::io::BufRead;
use std::sync::{Arc, Mutex, RwLock, Condvar};

use crossbeam_channel::{bounded, Receiver, Sender};

use lazy_static::lazy_static;

use rayon::ThreadPoolBuilder;

use num_cpus::get;

lazy_static! {
    static ref LINE_BUFFER_SIZE: usize = get() * get(); //(1 as u8).pow(2) as usize;
    static ref WORKER_COUNT: usize = get();
}

static RESULTS_BUFFER_SIZE: usize = 512;

pub fn read_async(r: Box<dyn BufRead + Send>) -> Receiver<Result<Element>> {
    let (element_sender, element_reciever) = bounded(RESULTS_BUFFER_SIZE);

    let interner = Interner::new();

    read_lines(interner, r, element_sender);

    element_reciever
}

fn read_lines(interner: Interner, mut r: Box<dyn BufRead + Send>, element_sender: Sender<Result<Element>>) {
    let (line_send, line_recv) = bounded::<(u64, Vec<u8>)>(*LINE_BUFFER_SIZE);
    let (results_send, results_recv) = bounded::<(u64, Result<Element>)>(*LINE_BUFFER_SIZE);

    let signal = Arc::new((Mutex::new(false), Condvar::new()));

    let pool = ThreadPoolBuilder::new().num_threads(*WORKER_COUNT).build().unwrap();

    let reader_done = Arc::new(RwLock::new(false));
    let worker_done = Arc::new(RwLock::new(false));

    {   
        let reader_done = reader_done.clone(); 
        // file reader thread
        std::thread::spawn(move || {
            let mut idx = 0 as u64;
            loop {
                let mut line = Vec::new();
                match r.read_until(b'\n', &mut line) {
                    Ok(_) => {
                        line_send.send((idx, line)).unwrap();
                    }
                    Err(_) => {
                        *reader_done.write().unwrap() = true;
                        return;
                    }
                }
                idx = (idx + 1) % *WORKER_COUNT as u64;
            }
        });
    }
    
    {    
        let worker_done = worker_done.clone();
        let signal = signal.clone();
        std::thread::spawn(move || {
            while !*reader_done.read().unwrap() {
                let (lock, sigvar) = &*signal;
                let mut ready = lock.lock().unwrap();

                pool.scope(|s| {
                    for _ in 0..*WORKER_COUNT {
                        let interner = interner.clone();
                        let line_recv = line_recv.clone();
                        let results_send = results_send.clone();
        
                        s.spawn(move |_| {
                            let (idx, line) = match line_recv.recv() {
                                Ok(line_pair) => line_pair,
                                Err(_) => return,
                            };
        
                            let element = deserialize_element(&interner, &line);
        
                            results_send.send((idx, element)).unwrap();
                        });
                    }
                });

                // set and signal aggregator
                *ready = true;
                sigvar.notify_one();

                // wait for signal from aggregator
                let mut ready = lock.lock().unwrap();
                while *ready {
                    ready = sigvar.wait(ready).unwrap();
                }
            }
            *worker_done.write().unwrap() = true;
        });
    }

    {    
        std::thread::spawn(move || {
            let mut elements = Vec::<Result<Element>>::with_capacity(*WORKER_COUNT);

            while !*worker_done.read().unwrap() {
                // wait for signal from worker manager
                let (lock, sigvar) = &*signal;
                let mut ready = lock.lock().unwrap();
                while !*ready {
                    ready = sigvar.wait(ready).unwrap();
                }

                for _ in 0..*WORKER_COUNT {
                    let el = match results_recv.recv() {
                        Ok(el) => el,
                        Err(_) => return
                    };

                    elements[el.0 as usize] = el.1;
                }

                for i in 0..*WORKER_COUNT {
                    let el_res = match &elements[i] {
                        Ok(el) => Ok(el.clone()),
                        Err(err) => Err(err.clone()),
                    };

                    element_sender.send(el_res).unwrap();
                }

                // reset and signal worker manager
                *ready = false;
                sigvar.notify_one();
            }
        });
    }
}