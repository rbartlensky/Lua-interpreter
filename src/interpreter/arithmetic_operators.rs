use bytecode::instructions::Value;
use bytecode::instructions::Value::*;

/// Converts the given values using the following rules:
/// 1. if one of the arguments is a float, then convert both args to Float.
/// 2. if one of the arguments is a string, then convert both args to Float.
/// 3. if both arguments are integers, then simply return two Integer values.
/// # Panics
/// This panics when one of the arguments is a Bool or a Nil.
fn to_int_or_float(lhs: &Value, rhs: &Value) -> (Value, Value) {
    let l;
    let r;
    if lhs.is_float() || rhs.is_float() {
        let msg = "Could not convert to float.";
        l = Float(lhs.to_float().expect(msg));
        r = Float(rhs.to_float().expect(msg));
    } else {
        let msg = "Could not convert to int.";
        l = Integer(lhs.to_int().expect(msg));
        r = Integer(rhs.to_int().expect(msg));
    }
    (l, r)
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
    match (lhs.to_float(), rhs.to_float()) {
        (Option::Some(l), Option::Some(r)) => Float(l / r),
        (_, _) => panic!("Argument could not be converted to float!")
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

pub fn exp(lhs: &Value, rhs: &Value) -> Value {
    match (lhs.to_float(), rhs.to_float()) {
        (Option::Some(l), Option::Some(r)) => Float(l.powf(r)),
        (_, _) => panic!("Argument could not be converted to float!")
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
            Float(1.0), Float(1.0), Float(1.0),
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

    #[test]
    fn test_exp() {
        let expected = vec![
            Float(4.0), Float(4.0), Float(4.0),
            Float(4.0), Float(4.0), Float(4.0),
            Float(4.0), Float(4.0), Float(4.0),
        ];
        test_operation(expected, exp);
    }
}
