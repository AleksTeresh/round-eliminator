// the file is adopted from https://crates.io/crates/cpython
use cpython::{PyResult, Python, py_module_initializer, py_fn};
use server::cli::get_complexity_sequential;

// add bindings to the generated python module
// N.B: names: "rust2py" must be the name of the `.so` or `.pyd` file
py_module_initializer!(rust2py, |py, m| {
    m.add(py, "__doc__", "This module is implemented in Rust.")?;
    m.add(
      py,
      "get_complexity",
      py_fn!(
        py,
        get_complexity_py(
          data: &str,
          labels: usize,
          iter: usize,
          merge : bool,
          autolb_features : &str,
          autoub_features : &str,
          pp_only: bool
        )
      )
    )?;
    Ok(())
});


// rust-cpython aware function. All of our python interface could be
// declared in a separate module.
// Note that the py_fn!() macro automatically converts the arguments from
// Python objects to Rust values; and the Rust return value back into a Python object.
fn get_complexity_py(
  _: Python,
  data: &str,
  labels: usize,
  iter: usize,
  merge : bool,
  autolb_features : &str,
  autoub_features : &str,
  pp_only: bool
) -> PyResult<(String, String)> {
    let out = get_complexity_sequential(
      data.to_string(),
      labels,
      iter,
      merge,
      autolb_features,
      autoub_features,
      pp_only
    );
    Ok(out)
}
