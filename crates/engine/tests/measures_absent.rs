//! A normal build has no realism measure: naming the team-shape sink, or the module that holds
//! every measure, fails to compile without the `measures` feature. The expected errors live
//! beside each case in `tests/ui_measures/*.stderr`; a toolchain change regenerates them with
//! `TRYBUILD=overwrite`. A `measures` build skips this test: its own tests use the sink.
#![cfg(not(feature = "measures"))]

#[test]
fn a_normal_build_has_no_shape_measure() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui_measures/*.rs");
}
