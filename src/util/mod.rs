use std::ptr;

pub mod random;

/// insert value into [T], which has one empty area on last.
/// ex) insert C at 1 into [A, B, uninit] => [A, C, B]
pub unsafe fn slice_insert<T>(ptr: &mut [T], index: usize, value: T) {
    let size = ptr.len();
    debug_assert!(size > index);

    let ptr = ptr.as_mut_ptr();

    if size > index + 1 {
        ptr::copy(ptr.add(index), ptr.add(index + 1), size - index - 1);
    }

    ptr::write(ptr.add(index), value);
}

/// remove value from [T] and remain last area without any init
/// ex) remove at 1 from [A, B, C] => [A, C, C(but you should not access here)]
pub unsafe fn slice_remove<T>(ptr: &mut [T], index: usize) -> T {
    let size = ptr.len();
    debug_assert!(size > index);

    let ptr = ptr.as_mut_ptr();
    let value = ptr::read(ptr.add(index));

    if size > index + 1 {
        ptr::copy(ptr.add(index + 1), ptr.add(index), size - index - 1);
    }

    value
}

#[macro_export]
macro_rules! ok_or {
    ($e:expr, $err:expr) => {{
        match $e {
            Ok(r) => r,
            Err(_) => $err,
        }
    }};
}

#[macro_export]
macro_rules! some_or {
    ($e:expr, $err:expr) => {{
        match $e {
            Some(r) => r,
            None => $err,
        }
    }};
}
