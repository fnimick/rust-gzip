#[doc="

    Module: macros

    This module contains some helpful macros used elsewhere
    throughout the codebase.
"]

#[macro_export]
pub macro_rules! bail {
    () => {
        return null::<c_void>() as *mut c_void;
    }
}

#[macro_export]
pub macro_rules! if_opt {
    ( $x:expr, $y:expr ) => {{
        if $x {
            Some($y)
        } else {
            None
        }
    }};
}

#[macro_export]
pub macro_rules! try_bail {
    ($expr: expr) => (match $expr {
        Option::Some(v) => v,
        Option::None => { bail!() },
    })
}

#[macro_export]
pub macro_rules! try_opt {
    ($expr:expr) => (match $expr {
        Option::Some(v) => v,
        Option::None => {
            return Option::None;
        }
    })
}

#[macro_export]
pub macro_rules! try_ref_opt {
    ($expr:expr) => (match $expr {
        &Option::Some(ref v) => v,
        &Option::None => {
            return Option::None;
        }
    })
}
