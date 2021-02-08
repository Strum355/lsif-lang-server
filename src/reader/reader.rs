use super::deserialize::deserialize_element;
use super::interner::Interner;
use super::types::*;

use std::io::BufRead;
use std::sync::{Arc, Condvar, Mutex, RwLock};

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

fn read_lines(
    interner: Interner,
    mut r: Box<dyn BufRead + Send>,
    element_sender: Sender<Result<Element>>,
) {
    let (line_send, line_recv) = bounded::<(u64, Vec<u8>)>(*LINE_BUFFER_SIZE);
    let (results_send, results_recv) = bounded::<(u64, Result<Element>)>(*LINE_BUFFER_SIZE);

    let signal = Arc::new((Mutex::new(false), Condvar::new()));

    let pool = ThreadPoolBuilder::new()
        .num_threads(*WORKER_COUNT)
        .build()
        .unwrap();

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
                        if line.is_empty() {
                            println!("done reading");
                            *reader_done.write().unwrap() = true;
                            return;
                        }
                        line_send.send((idx, line)).unwrap();
                    }
                    Err(_) => {
                        println!("done reading");
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
            let (lock, sigvar) = &*signal;
            while !*reader_done.read().unwrap() {
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
                            println!("sending a result");
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
            println!("worker done");
            *worker_done.write().unwrap() = true;
        });
    }

    {
        std::thread::spawn(move || {
            let mut elements = Vec::<Result<Element>>::with_capacity(*WORKER_COUNT);

            let (lock, sigvar) = &*signal;
            while !*worker_done.read().unwrap() {
                // wait for signal from worker manager
                let mut ready = lock.lock().unwrap();
                while !*ready {
                    ready = sigvar.wait(ready).unwrap();
                }

                for _ in 0..*WORKER_COUNT {
                    let el = match results_recv.recv() {
                        Ok(el) => el,
                        Err(_) => return,
                    };

                    println!("got a result");
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

#[cfg(test)]
mod test {
    use super::read_async;

    #[test]
    fn basic() {
        let string = r#"{ id: 2, type: "vertex", label: "project", kind: "typescript" }
{ id: 4, type: "vertex", label: "document", uri: "file:///home/burger/sample.ts", languageId: "typescript", contents: "..." }
{ id: 5, type: "vertex", label: "$event", kind: "begin", scope: "document" , data: 4 }
{ id: 3, type: "vertex", label: "$event", kind: "begin", scope: "project", data: 2 }
{ id: 53, type: "vertex", label: "$event", kind: "end", scope: "document", data: 4 }
{ id: 54, type: "edge", label: "contains", outV: 2, inVs: [4] }
{ id: 55, type: "vertex", label: "$event", kind: "end", scope: "project", data: 2 }"#;

        let chan = read_async(Box::new(string.as_bytes()));

        let mut count = 0;
        for _ in 0..7 {
            match chan.recv() {
                Ok(_) => println!("got an el"),
                Err(_) => break,
            };
            count += 1;
        }

        assert_eq!(count, 7);
    }
}
