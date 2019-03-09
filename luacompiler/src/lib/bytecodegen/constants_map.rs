use std::collections::HashMap;

/// Represents a structure which holds information about constants.
/// For each type (int, float, string) there is a corresponding constant table in
/// the bytecode.
/// This structure creates a mapping between constants used in the Lua source code
/// and their index in the constant table.
///
/// For instance, if '1.0' is used in Lua, then an entry from 1.0 to 0 is created in the
/// <float_map> member.
pub struct ConstantsMap {
    int_map: HashMap<i64, usize>,
    float_map: HashMap<String, usize>,
    str_map: HashMap<String, usize>,
}

impl ConstantsMap {
    /// Create an empty ConstantMap.
    pub fn new() -> ConstantsMap {
        ConstantsMap {
            int_map: HashMap::new(),
            float_map: HashMap::new(),
            str_map: HashMap::new(),
        }
    }

    /// Get the corresponding index of the given integer in the constant table.
    pub fn get_int(&mut self, int: i64) -> usize {
        let len = self.int_map.len();
        *self.int_map.entry(int).or_insert(len)
    }

    /// Get the integer constant table.
    pub fn get_ints(&self) -> Vec<i64> {
        let mut ints = Vec::with_capacity(self.int_map.len());
        ints.resize(self.int_map.len(), 0);
        for (&k, &v) in self.int_map.iter() {
            ints[v as usize] = k;
        }
        ints
    }

    /// Get the corresponding index of the given float in the constant table.
    pub fn get_float(&mut self, float: String) -> usize {
        let len = self.float_map.len();
        *self.float_map.entry(float).or_insert(len)
    }

    /// Get the float constant table.
    pub fn get_floats(&self) -> Vec<f64> {
        let mut floats = Vec::with_capacity(self.float_map.len());
        floats.resize(self.float_map.len(), 0.0);
        for (ref k, &v) in self.float_map.iter() {
            floats[v as usize] = k.parse().unwrap();
        }
        floats
    }

    /// Get the corresponding index of the given string in the constant table.
    pub fn get_str(&mut self, string: String) -> usize {
        let len = self.str_map.len();
        *self.str_map.entry(string).or_insert(len)
    }

    /// Get the strings constant table.
    pub fn get_strings(&self) -> Vec<String> {
        let mut strings = Vec::with_capacity(self.str_map.len());
        strings.resize(self.str_map.len(), String::from(""));
        for (ref k, &v) in self.str_map.iter() {
            // XXX: there should be a better way to do this...
            strings[v as usize] = k
                .to_string()
                .replace(r"\n", "\n")
                .replace(r"\t", "\t")
                .replace(r"\r", "\r");
        }
        strings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contants_map_works_correctly() {
        let mut cm = ConstantsMap::new();
        let ints = vec![2, 4, 1, 3];
        for &i in &ints {
            cm.get_int(i);
        }
        let floats = vec!["2.0", "4.2", "1.1", "3.0"];
        for &i in &floats {
            cm.get_float(i.to_string());
        }
        let strings = vec!["Foo", "Bar"];
        for &i in &strings {
            cm.get_str(i.to_string());
        }
        assert_eq!(cm.get_ints(), ints);
        assert_eq!(cm.get_floats().len(), floats.len());
        assert_eq!(cm.get_strings(), strings);
    }
}
