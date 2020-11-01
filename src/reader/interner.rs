use std::sync::Mutex;
use std::collections::HashMap;

pub struct Interner {
    map: Mutex<HashMap<String, u64>>,
}

impl Interner {
    pub fn new() -> Interner {
        Interner{
            map: Mutex::new(HashMap::new())
        }
    }

    pub fn intern(&self, raw: &[u8]) -> Result<u64, std::num::ParseIntError> {
        if raw.is_empty() {
            return Ok(0)
        }

        if raw[0] != b'"' {
            unsafe {
                return String::from_utf8_unchecked(raw.to_vec()).parse::<u64>()
            }
        }

        unsafe {
            let s = String::from_utf8_unchecked(raw[1..raw.len()-1].to_vec());

            match s.parse::<u64>() {
                Ok(num) => return Ok(num),
                Err(_) => {}
            }
            
            let mut map = self.map.lock().unwrap();
            if map.contains_key(&s) {
                return Ok(*map.get(&s).unwrap());
            }

            let id: u64 = (map.len()+1) as u64;
            map.insert(s, id);
            Ok(id)
        }
    }
}


#[test]
fn it_works() {
    let interner = Interner::new();

    interner.intern(&Vec::new()).unwrap();
}
