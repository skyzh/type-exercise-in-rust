use crate::{ScalarImpl, ScalarRefImpl};

#[test]
fn checkpoint_2_erases_and_recovers_scalars() {
    assert_eq!(i32::try_from(ScalarImpl::from(9_i32)).unwrap(), 9);
    assert_eq!(
        String::try_from(ScalarImpl::from(String::from("owned"))).unwrap(),
        "owned"
    );
    assert_eq!(i32::try_from(ScalarRefImpl::from(7_i32)).unwrap(), 7);
    assert_eq!(
        <&str>::try_from(ScalarRefImpl::from("borrowed")).unwrap(),
        "borrowed"
    );

    assert!(i32::try_from(ScalarImpl::String("wrong".to_owned())).is_err());
    assert!(<&str>::try_from(ScalarRefImpl::Int32(1)).is_err());
}

// === Chapter 1 checkpoint 3 ===
