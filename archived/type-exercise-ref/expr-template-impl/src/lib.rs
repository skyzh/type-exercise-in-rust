// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

use anyhow::Result;
use itertools::Itertools;
use quote::{format_ident, quote};

pub fn generate_expression_template(param_number: usize) -> Result<String> {
    let expr_template_name = format_ident!("FnArgs{}Expression", param_number);
    let gp = (0..param_number)
        .map(|i| format_ident!("I{}", i + 1))
        .collect_vec();
    let it = (0..param_number)
        .map(|i| format_ident!("i{}", i + 1))
        .collect_vec();
    let position = 0..param_number;

    let impl_before = quote! {
        #( #gp, )* O, F
    };

    let struct_opts = quote! {
        #expr_template_name<#( #gp, )* O, F>
    };

    let bounds = quote! {
        O: Scalar,
        #( #gp: Scalar, )*
        F: Fn(
            #( #gp::RefType<'_>, )*
        ) -> O,
    };

    let extra_bounds = quote! {
        #( for<'a> &'a #gp::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>, )*
    };

    let tokens = quote! {
        use crate::common::*;

        /// Represents an expression which takes `Ix` as input parameter, and outputs scalar
        /// of type `O`.
        ///
        /// `ArgsNExpression` automatically vectorizes the scalar function to a vectorized one, while
        /// erasing the concreate array type. Therefore, users simply call
        /// `ArgsNExpression::eval(ArrayImpl, ArrayImpl)`, while developers only need to provide
        /// implementation for functions like `cmp_le(i32, i32)`.
        pub struct #expr_template_name<#impl_before> where #bounds {
            func: F,
            _phantom: PhantomData<(#( #gp, )* O)>,
        }

        /// Implement `ArgsNExpression` for any given scalar function `F`.
        ///
        /// Note that as we cannot add `From<&'a ArrayImpl>` bound on [`Array`], so we have to specify them
        /// here.
        impl<#impl_before> #struct_opts
        where
            #bounds
            #extra_bounds
        {
            /// Create an expression from existing function
            pub fn new(func: F) -> Self {
                Self {
                    func,
                    _phantom: PhantomData,
                }
            }

            /// Evaluate the expression with the given array.
            pub fn eval_batch(&self, #( #it: &ArrayImpl),*) -> Result<ArrayImpl> {
                #(
                    let #it: &#gp::ArrayType = #it.try_into()?;
                )*
                #(
                    assert_eq!(i1.len(), #it.len(), "array length mismatch");
                )*
                let mut builder = <O::ArrayType as Array>::Builder::with_capacity(i1.len());
                for ( #( #it ),* ) in itertools::izip!(
                    #( #it.iter() ),*
                ) {
                    match ( #( #it, )* ) {
                        ( #( Some(#it), )* ) => builder.push(Some((self.func)(#( #it, )*).as_scalar_ref())),
                        _ => builder.push(None),
                    }
                }
                Ok(builder.finish().into())
            }
        }

        /// Blanket [`Expression`] implementation for `ArgsNExpression`
        impl<#impl_before> Expression for #struct_opts
        where
            #bounds
            #extra_bounds
        {
            fn eval_expr(&self, data: &[&ArrayImpl]) -> Result<ArrayImpl> {
                if data.len() != #param_number {
                    return Err(anyhow!("Expect {} inputs for {}", #param_number, stringify!(#expr_template_name)));
                }
                self.eval_batch(
                    #(data[ #position ],)*
                )
            }
        }
    };

    let syntax_tree = syn::parse_file(tokens.to_string().as_str())?;

    let func_template = prettyplease::unparse(&syntax_tree);

    Ok(func_template)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_binary_expression() {
        println!("{}", generate_expression_template(2).unwrap());
    }
}
