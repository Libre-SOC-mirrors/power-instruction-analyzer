// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

#![cfg(feature = "python")]

use crate::{DivInput, DivResult, OverflowFlags};
use pyo3::{prelude::*, wrap_pyfunction, PyObjectProtocol};
use std::{borrow::Cow, cell::RefCell, fmt};

trait ToPythonRepr {
    fn to_python_repr(&self) -> Cow<str> {
        struct Helper<T>(RefCell<Option<T>>);

        impl<T: FnOnce(&mut fmt::Formatter<'_>) -> fmt::Result> fmt::Display for Helper<T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.borrow_mut().take().unwrap()(f)
            }
        }

        impl<T: FnOnce(&mut fmt::Formatter<'_>) -> fmt::Result> Helper<T> {
            fn new(f: T) -> Self {
                Helper(RefCell::new(Some(f)))
            }
        }
        Cow::Owned(format!(
            "{}",
            Helper::new(|f: &mut fmt::Formatter<'_>| -> fmt::Result { self.write(f) })
        ))
    }
    fn write(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_python_repr())
    }
}

fn write_list_body_to_python_repr<I: IntoIterator<Item = T>, T: ToPythonRepr>(
    list: I,
    f: &mut fmt::Formatter<'_>,
    separator: &str,
) -> fmt::Result {
    let mut first = true;
    for i in list {
        if first {
            first = false;
        } else {
            f.write_str(separator)?;
        }
        i.write(f)?;
    }
    Ok(())
}

struct NamedArgPythonRepr<'a> {
    name: &'a str,
    value: &'a (dyn ToPythonRepr + 'a),
}

impl ToPythonRepr for NamedArgPythonRepr<'_> {
    fn write(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)?;
        f.write_str("=")?;
        self.value.write(f)
    }
}

impl<T: ToPythonRepr> ToPythonRepr for &'_ T {
    fn to_python_repr(&self) -> Cow<str> {
        (**self).to_python_repr()
    }
    fn write(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).write(f)
    }
}

impl ToPythonRepr for bool {
    fn to_python_repr(&self) -> Cow<str> {
        Cow::Borrowed(match self {
            true => "True",
            false => "False",
        })
    }
}

impl<T: ToPythonRepr> ToPythonRepr for Option<T> {
    fn to_python_repr(&self) -> Cow<str> {
        match self {
            Some(v) => v.to_python_repr(),
            None => Cow::Borrowed("None"),
        }
    }
    fn write(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Some(v) => v.write(f),
            None => f.write_str("None"),
        }
    }
}

impl<T: ToPythonRepr> ToPythonRepr for Vec<T> {
    fn write(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        write_list_body_to_python_repr(self, f, ", ")?;
        f.write_str("]")
    }
}

macro_rules! impl_int_to_python_repr {
    ($($int:ident,)*) => {
        $(
            impl ToPythonRepr for $int {
                fn write(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    write!(f, "{}", self)
                }
            }
        )*
    };
}

impl_int_to_python_repr! {u8, u16, u32, u64, u128, i8, i16, i32, i64, i128,}

macro_rules! wrap_type {
    (
        #[pymodule($m:expr)]
        // use tt to work around PyO3 bug fixed in PyO3#832
        #[pyclass $($pyclass_args:tt)?]
        #[wrapped($value:ident: $wrapped:ident)]
        $(#[$meta:meta])*
        struct $wrapper:ident {
            $(
                #[set=$setter_name:ident]
                $(#[$field_meta:meta])*
                $field_name:ident:$field_type:ty,
            )*
        }
    ) => {
        #[pyclass $($pyclass_args)?]
        $(#[$meta])*
        #[derive(Clone)]
        struct $wrapper {
            $value: $wrapped,
        }

        impl<'source> FromPyObject<'source> for $wrapped {
            fn extract(ob: &'source PyAny) -> PyResult<Self> {
                Ok(ob.extract::<$wrapper>()?.$value)
            }
        }

        impl IntoPy<PyObject> for $wrapped {
            fn into_py(self, py: Python) -> PyObject {
                $wrapper { $value: self }.into_py(py)
            }
        }

        #[pymethods]
        impl $wrapper {
            #[new]
            fn new($($field_name:$field_type),*) -> Self {
                Self {
                    $value: $wrapped {
                        $($field_name),*
                    }
                }
            }
            $(
                #[getter]
                $(#[$field_meta:meta])*
                fn $field_name(&self) -> $field_type {
                    self.$value.$field_name
                }
                #[setter]
                fn $setter_name(&mut self, $field_name: $field_type) {
                    self.$value.$field_name = $field_name;
                }
            )*
        }

        impl ToPythonRepr for $wrapped {
            fn write(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($wrapped), "("))?;
                write_list_body_to_python_repr(&[
                    $(
                        NamedArgPythonRepr {
                            name: stringify!($field_name),
                            value: &self.$field_name,
                        },
                    )*
                    ], f, ", ")?;
                f.write_str(")")
            }
        }

        #[pyproto]
        impl PyObjectProtocol for $wrapper {
            fn __str__(&self) -> String {
                serde_json::to_string(&self.$value).unwrap()
            }
            fn __repr__(&self) -> String {
                self.$value.to_python_repr().into_owned()
            }
        }

        $m.add_class::<$wrapper>()?;
    };
}

macro_rules! wrap_instr_fns {
    (
        #![pymodule($m:ident)]
        $(
            // use tt to work around PyO3 bug fixed in PyO3#832
            $(#[pyfunction $pyfunction_args:tt])?
            $(#[$meta:meta])*
            fn $name:ident(inputs: $inputs:ty) -> $result:ty;
        )*
    ) => {
        $(
            {
                #[pyfunction $($pyfunction_args)?]
                #[text_signature = "(inputs)"]
                $(#[$meta])*
                fn $name(inputs: $inputs) -> $result {
                    $crate::instr_models::$name(inputs)
                }

                $m.add_wrapped(wrap_pyfunction!($name))?;
            }
        )*
    };
}

#[pymodule]
fn power_instruction_analyzer(_py: Python, m: &PyModule) -> PyResult<()> {
    wrap_type! {
        #[pymodule(m)]
        #[pyclass(name = OverflowFlags)]
        #[wrapped(value: OverflowFlags)]
        #[text_signature = "(overflow, overflow32)"]
        struct PyOverflowFlags {
            #[set = set_overflow]
            overflow: bool,
            #[set = set_overflow32]
            overflow32: bool,
        }
    }

    wrap_type! {
        #[pymodule(m)]
        #[pyclass(name = DivInput)]
        #[wrapped(value: DivInput)]
        #[text_signature = "(dividend, divisor, result_prev)"]
        struct PyDivInput {
            #[set = set_dividend]
            dividend: u64,
            #[set = set_divisor]
            divisor: u64,
            #[set = set_result_prev]
            result_prev: u64,
        }
    }

    wrap_type! {
        #[pymodule(m)]
        #[pyclass(name = DivResult)]
        #[wrapped(value: DivResult)]
        #[text_signature = "(result, overflow)"]
        struct PyDivResult {
            #[set = set_result]
            result: u64,
            #[set = set_overflow]
            overflow: Option<OverflowFlags>,
        }
    }
    wrap_instr_fns! {
        #![pymodule(m)]

        fn divdeo(inputs: DivInput) -> DivResult;
        fn divdeuo(inputs: DivInput) -> DivResult;
        fn divdo(inputs: DivInput) -> DivResult;
        fn divduo(inputs: DivInput) -> DivResult;
        fn divweo(inputs: DivInput) -> DivResult;
        fn divweuo(inputs: DivInput) -> DivResult;
        fn divwo(inputs: DivInput) -> DivResult;
        fn divwuo(inputs: DivInput) -> DivResult;
        fn modsd(inputs: DivInput) -> DivResult;
        fn modud(inputs: DivInput) -> DivResult;
        fn modsw(inputs: DivInput) -> DivResult;
        fn moduw(inputs: DivInput) -> DivResult;
    }
    Ok(())
}
