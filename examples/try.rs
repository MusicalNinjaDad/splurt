#![feature(yeet_expr)]
#![deny(unfulfilled_lint_expectations)]

fn main() -> Result<(), i32> {
    Err(4)?;
    do yeet 4;
    #[expect(unreachable_code)]
    Ok(())
}
