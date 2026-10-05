//! The compiler refuses a module that writes match state: writing through the read-only view
//! and reaching the match behind it both fail to compile, and a module that only reads the
//! view compiles. The expected errors live beside each case in `tests/ui/*.stderr`; a
//! toolchain change regenerates them with `TRYBUILD=overwrite`.

#[test]
fn a_module_cannot_write_match_state() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/writes_*.rs");
    cases.pass("tests/ui/reads_state.rs");
}
