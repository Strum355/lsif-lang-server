use std::collections::HashMap;
use std::marker::Sync;
use std::sync::{Arc, Mutex};

/// Interner converts strings into unique identifers. Submitting the same byte value to
/// the interner will result in the same identifier being produced. Each unique input is
/// guaranteed to have a unique output (no two inputs share the same identifier). The
/// identifier space of two distinct interner instances may overlap.
///
/// Assumption: The output of LSIF indexers will not generally mix types of identifiers.
/// If integers are used, they are used for all ids. If strings are used, they are used
/// for all ids.
#[derive(Clone)]
pub struct Interner {
    map: Arc<Mutex<HashMap<String, u64>>>,
}

unsafe impl Sync for Interner {}

impl Interner {
    pub fn new() -> Interner {
        Interner {
            map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Intern returns the unique identifier for the given byte value. The byte value should
    /// be a raw LSIF input identifier, which should be a JSON-encoded number or quoted string.
    /// This method is safe to call from multiple goroutines.
    pub fn intern(&self, raw: &[u8]) -> Result<u64, std::num::ParseIntError> {
        if raw.is_empty() {
            return Ok(0);
        }

        if raw[0] != b'"' {
            unsafe { return String::from_utf8_unchecked(raw.to_vec()).parse::<u64>() }
        }

        let s = unsafe { String::from_utf8_unchecked(raw[1..raw.len() - 1].to_vec()) };

        match s.parse::<u64>() {
            Ok(num) => return Ok(num),
            Err(_) => {}
        }

        let mut map = self.map.lock().unwrap();
        if map.contains_key(&s) {
            return Ok(*map.get(&s).unwrap());
        }

        let id: u64 = (map.len() + 1) as u64;
        map.insert(s, id);
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{format_err, Result};
    use std::collections::HashSet;

    fn compare_from_vec(input: &[Vec<u8>]) -> Result<HashSet<u64>> {
        let mut results = HashSet::with_capacity(input.len());
        let interner = Interner::new();

        for num in input {
            let x = interner.intern(num).unwrap();
            results.insert(x);
        }

        for num in input {
            let x = interner.intern(num).unwrap();
            results
                .get(&x)
                .ok_or(format_err!("result not found in previous set"))?;
        }

        Ok(results)
    }

    fn string_vec_to_bytes(values: Vec<&str>) -> Vec<Vec<u8>> {
        values.iter().map(|s| s.as_bytes().to_vec()).collect()
    }

    #[test]
    fn empty_data_doesnt_throw() {
        let interner = Interner::new();

        assert_eq!(interner.intern(&Vec::new()).unwrap(), 0);
    }

    #[test]
    fn numbers_test() {
        let values = string_vec_to_bytes(vec!["1", "2", "3", "4", "100", "500"]);

        let results = compare_from_vec(&values).unwrap();

        assert_eq!(results.len(), values.len());
    }

    #[test]
    fn numbers_in_strings_test() {
        let values = string_vec_to_bytes(vec![
            r#""1""#, r#""2""#, r#""3""#, r#""4""#, r#""100""#, r#""500""#,
        ]);

        let results = compare_from_vec(&values).unwrap();

        assert_eq!(results.len(), values.len());
    }

    #[test]
    fn normal_strings_test() {
        let values = string_vec_to_bytes(vec![
            r#""assert_eq!(results.len(), values.len());""#,
            r#""sample text""#,
            r#""why must this be utf16. Curse you javascript""#,
            r#""I'd just like to interject for a moment.  What you're referring to as Linux,
            is in fact, GNU/Linux, or as I've recently taken to calling it, GNU plus Linux.
            Linux is not an operating system unto itself, but rather another free component
            of a fully functioning GNU system made useful by the GNU corelibs, shell
            utilities and vital system components comprising a full OS as defined by POSIX.""#,
        ]);

        let results = compare_from_vec(&values).unwrap();

        assert_eq!(results.len(), values.len());
    }

    #[test]
    fn duplicate_string() {
        let values = string_vec_to_bytes(vec![
            r#""assert_eq!(results.len(), values.len());""#,
            r#""sample text""#,
            r#""why must this be utf16. Curse you javascript""#,
            r#""why must this be utf16. Curse you javascript""#,
            r#""why must this be utf16. Curse you javascript""#,
            r#""I'd just like to interject for a moment.  What you're referring to as Linux,
            is in fact, GNU/Linux, or as I've recently taken to calling it, GNU plus Linux.
            Linux is not an operating system unto itself, but rather another free component
            of a fully functioning GNU system made useful by the GNU corelibs, shell
            utilities and vital system components comprising a full OS as defined by POSIX.""#,
        ]);

        let results = compare_from_vec(&values).unwrap();

        assert_eq!(results.len(), values.len() - 2);
    }
}
