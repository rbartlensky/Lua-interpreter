use bytecode::instructions::Value;

#[derive(Clone, Debug)]
pub struct Reg {
    value: Value
}

impl Reg {
    pub fn new() -> Reg {
        Reg { value: Value::Nil }
    }

    pub fn get_value(&self) -> &Value {
        &self.value
    }

    pub fn set_value(&mut self, value: Value) {
        self.value = value;
    }
}
