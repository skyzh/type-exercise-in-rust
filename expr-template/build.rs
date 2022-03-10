// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

use std::fmt::Write;

use anyhow::Result;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=../expr-tempalte-impl");

    let mut gen_header = String::new();

    writeln!(gen_header, "#![allow(dead_code)]")?;
    writeln!(gen_header, "#![allow(unused_parens)]")?;
    writeln!(gen_header)?;

    for i in 1..=5 {
        let content = expr_template_impl::generate_expression_template(i)?;
        std::fs::write(format!("src/gen/fn_args_{}_expression.rs", i), content)?;
        writeln!(gen_header, "mod fn_args_{}_expression;", i)?;
        writeln!(gen_header, "pub use fn_args_{}_expression::*;", i)?;
    }

    std::fs::write("src/gen/mod.rs", gen_header)?;

    Ok(())
}
