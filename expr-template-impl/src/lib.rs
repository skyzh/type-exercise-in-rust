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
    let vp = (0..param_number)
        .map(|i| format_ident!("V{}", i + 1))
        .collect_vec();
    let position = 0..param_number;
    let view_kinds = [
        format_ident!("Array"),
        format_ident!("Constant"),
        format_ident!("Dictionary"),
    ];
    let dispatch_arms = (0..param_number)
        .map(|_| view_kinds.iter().cloned())
        .multi_cartesian_product()
        .map(|kinds| {
            let patterns = it
                .iter()
                .zip(kinds)
                .map(|(input, kind)| quote! { ColumnView::#kind(#input) });
            quote! {
                (#(#patterns,)*) => self.eval_typed(#(#it),*)
            }
        })
        .collect_vec();

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
        ) -> O + Send + Sync,
    };

    let extra_bounds = quote! {
        #( for<'a> &'a #gp::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>, )*
        #( for<'a> #gp::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>, )*
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

            #[inline]
            fn eval_typed<'a, #(#vp,)*>(&self, #(#it: #vp),*) -> Result<ArrayImpl>
            where
                #(#vp: ColumnAccessor<'a, #gp>,)*
            {
                let len = i1.len();
                #(
                    if #it.len() != len {
                        return Err(anyhow!(
                            "column length mismatch: expected {}, got {}",
                            len,
                            #it.len(),
                        ));
                    }
                )*
                let mut builder = <O::ArrayType as Array>::Builder::with_capacity(len);
                for row in 0..len {
                    match ( #( #it.get(row), )* ) {
                        ( #( Some(#it), )* ) => {
                            builder.push(Some((self.func)(#( #it, )*).as_scalar_ref()))
                        }
                        _ => builder.push(None),
                    }
                }
                Ok(builder.finish().into())
            }

            /// Evaluate the expression over logical column views.
            pub fn eval_views<'a>(&self, #( #it: ColumnViewImpl<'a>),*) -> Result<ArrayImpl> {
                #(
                    let #it = ColumnView::<#gp>::try_from(#it)?;
                )*
                match (#(#it,)*) {
                    #(#dispatch_arms,)*
                }
            }

            /// Evaluate regular arrays through the column-view compatibility adapter.
            pub fn eval_batch(&self, #( #it: &ArrayImpl),*) -> Result<ArrayImpl> {
                self.eval_views(
                    #(ColumnViewImpl::array(#it),)*
                )
            }
        }

        /// Blanket [`Expression`] implementation for `ArgsNExpression`
        impl<#impl_before> Expression for #struct_opts
        where
            #bounds
            #extra_bounds
        {
            fn eval(&self, data: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl> {
                if data.len() != #param_number {
                    return Err(anyhow!("expected {} inputs for {}", #param_number, stringify!(#expr_template_name)));
                }
                self.eval_views(
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
