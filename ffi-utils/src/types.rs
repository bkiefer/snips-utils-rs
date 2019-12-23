use std::ffi::CString;
use std::ptr::null;

use failure::{Error, ResultExt};

use crate::conversions::*;
use crate::convert_to_c_string_result;
use crate::create_rust_string_from;

/// Used as a return type of functions that can encounter errors
#[repr(C)]
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
#[allow(non_camel_case_types)]
pub enum SNIPS_RESULT {
    /// The function returned successfully
    SNIPS_RESULT_OK = 0,
    /// The function encountered an error, you can retrieve it using the dedicated function
    SNIPS_RESULT_KO = 1,
}

/// An array of strings
#[repr(C)]
#[derive(Debug)]
pub struct CStringArray {
    /// Pointer to the first element of the array
    pub data: *const *const libc::c_char,
    /// Number of elements in the array
    // Note: we can't use `libc::size_t` because it's not supported by JNA
    pub size: libc::c_int,
}

unsafe impl Sync for CStringArray {}

impl AsRust<Vec<String>> for CStringArray {
    fn as_rust(&self) -> Result<Vec<String>, Error> {
        let mut result = vec![];

        let strings = unsafe {
            std::slice::from_raw_parts_mut(self.data as *mut *mut libc::c_char, self.size as usize)
        };

        for s in strings {
            result.push(create_rust_string_from!(*s))
        }

        Ok(result)
    }
}

impl CReprOf<Vec<String>> for CStringArray {
    fn c_repr_of(input: Vec<String>) -> Result<Self, Error> {
        Ok(Self {
            size: input.len() as libc::c_int,
            data: Box::into_raw(
                input
                    .into_iter()
                    .map(|s| convert_to_c_string_result!(s))
                    .collect::<Result<Vec<*const libc::c_char>, _>>()
                    .context("Could not convert Vector of Strings to C Repr")?
                    .into_boxed_slice(),
            ) as *const *const libc::c_char,
        })
    }
}

impl Drop for CStringArray {
    fn drop(&mut self) {
        let _ = unsafe {
            let y = Box::from_raw(std::slice::from_raw_parts_mut(
                self.data as *mut *mut libc::c_char,
                self.size as usize,
            ));
            for p in y.into_iter() {
                let _ = CString::from_raw_pointer(*p); // let's not panic if we fail here
            }
        };
    }
}

#[repr(C)]
pub struct CArray<T> {
    data_ptr: *const T,
    size: usize,
}

impl<U: AsRust<V>, V> AsRust<Vec<V>> for CArray<U> {
    fn as_rust(&self) -> Result<Vec<V>, Error> {
        let mut vec = Vec::with_capacity(self.size);
        if self.size > 0 {
            let values = unsafe { std::slice::from_raw_parts_mut(self.data_ptr as *mut U, self.size) };
            for value in values {
                vec.push(value.as_rust()?);
            }
        }
        Ok(vec)
    }
}

impl<U: CReprOf<V>, V> CReprOf<Vec<V>> for CArray<U> {
    fn c_repr_of(input: Vec<V>) -> Result<Self, Error> {
        let input_size = input.len();
        Ok(
            Self {
                data_ptr: if input_size > 0 {
                    Box::into_raw(
                        input
                            .into_iter()
                            .map(|item| U::c_repr_of(item))
                            .collect::<Result<Vec<_>, Error>>()
                            .expect("Could not convert to C representation")
                            .into_boxed_slice()
                    ) as *const U
                } else {
                    null() as *const U
                },
                size: input_size
            }
        )
    }
}

impl<T> Drop for CArray<T> {
    fn drop(&mut self) {
        let _ = unsafe {
            Box::from_raw(std::slice::from_raw_parts_mut(
                self.data_ptr as *mut T,
                self.size,
            ))
        };
    }
}
