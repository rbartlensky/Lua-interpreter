use bytecode::instructions::Value;
use bytecode::instructions::Value::*;

/// Converts the given values using the following rules:
/// 1. if one of the arguments is a float, then convert both args to Float.
/// 2. if one of the arguments is a string, then convert both args to Float.
/// 3. if both arguments are integers, then simply return two Integer values.
/// # Panics
/// This panics when one of the arguments is a Bool or a Nil.
fn to_int_or_float(lhs: &Value, rhs: &Value) -> (Value, Value) {
    match (lhs, rhs) {
        (Float(l), Float(r)) => (Float(*l), Float(*r)),
        (Float(l), Integer(r)) => (Float(*l), Float(*r as f64)),
        (Float(l), Str(r)) => (Float(*l), Float(r.parse().unwrap())),
        (Integer(l), Float(r)) => (Float(*l as f64), Float(*r)),
        (Integer(l), Integer(r)) => (Integer(*l), Integer(*r)),
        (Integer(l), Str(r)) => (Float(*l as f64), Float(r.parse().unwrap())),
        (Str(l), Float(r)) => (Float(l.parse().unwrap()), Float(*r)),
        (Str(l), Integer(r)) => (Float(l.parse().unwrap()), Float(*r as f64)),
        (Str(l), Str(r)) => (Float(l.parse().unwrap()), Float(r.parse().unwrap())),
        (_, _ ) => panic!("Cannot convert to float or int {}, {}", lhs, rhs)
    }
}

pub fn add(lhs: &Value, rhs: &Value) -> Value {
    match to_int_or_float(lhs, rhs) {
        (Float(l), Float(r)) => Float(l + r),
        (Integer(l), Integer(r)) => Integer(l + r),
        (_, _) => unreachable!()
    }
}

pub fn sub(lhs: &Value, rhs: &Value) -> Value {
    match to_int_or_float(lhs, rhs) {
        (Float(l), Float(r)) => Float(l - r),
        (Integer(l), Integer(r)) => Integer(l - r),
        (_, _) => unreachable!()
    }
}

pub fn mul(lhs: &Value, rhs: &Value) -> Value {
    match to_int_or_float(lhs, rhs) {
        (Float(l), Float(r)) => Float(l * r),
        (Integer(l), Integer(r)) => Integer(l * r),
        (_, _) => unreachable!()
    }
}

pub fn div(lhs: &Value, rhs: &Value) -> Value {
    match to_int_or_float(lhs, rhs) {
        (Float(l), Float(r)) => Float(l / r),
        (Integer(l), Integer(r)) => Integer(l / r),
        (_, _) => unreachable!()
    }
}

pub fn modulus(lhs: &Value, rhs: &Value) -> Value {
    match to_int_or_float(lhs, rhs) {
        (Float(l), Float(r)) => Float(l % r),
        (Integer(l), Integer(r)) => Integer(l % r),
        (_, _) => unreachable!()
    }
}

pub fn fdiv(lhs: &Value, rhs: &Value) -> Value {
    match to_int_or_float(lhs, rhs) {
        (Float(l), Float(r)) => Float((l / r).floor()),
        (Integer(l), Integer(r)) => Integer(l / r),
        (_, _) => unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_operation(expected: Vec<Value>, op: fn(&Value, &Value) -> Value) {
        let test_cases = vec![
            op(&Integer(2), &Integer(2)),
            op(&Integer(2), &Float(2.0)),
            op(&Integer(2), &Str(String::from("2"))),
            op(&Float(2.0), &Integer(2)),
            op(&Float(2.0), &Float(2.0)),
            op(&Float(2.0), &Str(String::from("2"))),
            op(&Str(String::from("2")), &Integer(2)),
            op(&Str(String::from("2")), &Float(2.0)),
            op(&Str(String::from("2")), &Str(String::from("2")))
        ];
        assert_eq!(test_cases, expected);
    }

    #[test]
    fn test_add() {
        let expected = vec![
            Integer(4), Float(4.0), Float(4.0),
            Float(4.0), Float(4.0), Float(4.0),
            Float(4.0), Float(4.0), Float(4.0),
        ];
        test_operation(expected, add);
    }

    #[test]
    fn test_sub() {
        let expected = vec![
            Integer(0), Float(0.0), Float(0.0),
            Float(0.0), Float(0.0), Float(0.0),
            Float(0.0), Float(0.0), Float(0.0),
        ];
        test_operation(expected, sub);
    }

    #[test]
    fn test_mul() {
        let expected = vec![
            Integer(4), Float(4.0), Float(4.0),
            Float(4.0), Float(4.0), Float(4.0),
            Float(4.0), Float(4.0), Float(4.0),
        ];
        test_operation(expected, mul);
    }

    #[test]
    fn test_div() {
        let expected = vec![
            Integer(1), Float(1.0), Float(1.0),
            Float(1.0), Float(1.0), Float(1.0),
            Float(1.0), Float(1.0), Float(1.0),
        ];
        test_operation(expected, div);
    }

    #[test]
    fn test_modulus() {
        let expected = vec![
            Integer(0), Float(0.0), Float(0.0),
            Float(0.0), Float(0.0), Float(0.0),
            Float(0.0), Float(0.0), Float(0.0),
        ];
        test_operation(expected, modulus);
    }

    #[test]
    fn test_floor_div() {
        let expected = vec![
            Integer(1), Float(1.0), Float(1.0),
            Float(1.0), Float(1.0), Float(1.0),
            Float(1.0), Float(1.0), Float(1.0),
        ];
        test_operation(expected, fdiv);
    }
}
