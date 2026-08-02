/// An owned scalar whose concrete type is known only at runtime.
#[derive(Clone, Debug, PartialEq)]
pub enum ScalarImpl {
    Int32(i32),
    String(String),
}

#[cfg(test)]
mod tests {
    use super::ScalarImpl;

    #[test]
    fn starter_distinguishes_the_two_owned_scalar_variants() {
        assert_eq!(ScalarImpl::Int32(7), ScalarImpl::Int32(7));
        assert_eq!(
            ScalarImpl::String("rust".to_owned()),
            ScalarImpl::String("rust".to_owned())
        );
        assert_ne!(ScalarImpl::Int32(7), ScalarImpl::String("7".to_owned()));
    }
}
