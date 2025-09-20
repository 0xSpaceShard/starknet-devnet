/// Asserts that a boolean expression is `true` at runtime, propagating an `Err` if it's false.
#[macro_export]
macro_rules! assert_prop {
    ($cond:expr) => {
        if $cond {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Assertion failed: {}",
                stringify!($cond)
            ))
        }
    };
    ($cond:expr, $($arg:tt)+) => {
        if $cond {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Assertion failed: {}: {}",
                stringify!($cond),
                format!($($arg)+)
            ))
        }
    };
}

/// Asserts that two expressions are equal to each other, propagating an `Err` if they are not.
#[macro_export]
macro_rules! assert_eq_prop {
    ($left:expr, $right:expr) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if *left_val == *right_val {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Assertion failed: `(left == right)`
  left: `{:?}`,
 right: `{:?}`",
                        left_val,
                        right_val
                    ))
                }
            }
        }
    };
    ($left:expr, $right:expr, $($arg:tt)+) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if *left_val == *right_val {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Assertion failed: `(left == right)`: {}
  left: `{:?}`,
 right: `{:?}`",
                        format!($($arg)+),
                        left_val,
                        right_val
                    ))
                }
            }
        }
    };
}

/// Asserts that two expressions are not equal to each other, propagating an `Err` if they are.
#[macro_export]
macro_rules! assert_ne_prop {
    ($left:expr, $right:expr) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if *left_val != *right_val {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Assertion failed: `(left != right)`
  left: `{:?}`,
 right: `{:?}`",
                        left_val,
                        right_val
                    ))
                }
            }
        }
    };
    ($left:expr, $right:expr, $($arg:tt)+) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if *left_val != *right_val {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Assertion failed: `(left != right)`: {}
  left: `{:?}`,
 right: `{:?}`",
                        format!($($arg)+),
                        left_val,
                        right_val
                    ))
                }
            }
        }
    };
}

/// Asserts that an expression is greater than or equal to the other, propagating an `Err` if they are not.
#[macro_export]
macro_rules! assert_gte_prop {
    ($left:expr, $right:expr) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if *left_val >= *right_val {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Assertion failed: `(left >= right)`
  left: `{:?}`,
 right: `{:?}`",
                        left_val,
                        right_val
                    ))
                }
            }
        }
    };
    ($left:expr, $right:expr, $($arg:tt)+) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if *left_val >= *right_val {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Assertion failed: `(left >= right)`: {}
  left: `{:?}`,
 right: `{:?}`",
                        format!($($arg)+),
                        left_val,
                        right_val
                    ))
                }
            }
        }
    };
}

/// Asserts that an expression is less than or equal to the other, propagating an `Err` if they are not.
#[macro_export]
macro_rules! assert_lte_prop {
    ($left:expr, $right:expr) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if *left_val <= *right_val {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Assertion failed: `(left <= right)`
  left: `{:?}`,
 right: `{:?}`",
                        left_val,
                        right_val
                    ))
                }
            }
        }
    };
    ($left:expr, $right:expr, $($arg:tt)+) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if *left_val <= *right_val {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Assertion failed: `(left <= right)`: {}
  left: `{:?}`,
 right: `{:?}`",
                        format!($($arg)+),
                        left_val,
                        right_val
                    ))
                }
            }
        }
    };
}
