use crate::DataType;

/// One ordered, lossless implicit numeric promotion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NumericPromotion {
    pub left: DataType,
    pub right: DataType,
    pub output: DataType,
}

/// The complete implicit numeric-promotion policy.
///
/// Both operand orders are listed deliberately. `Integer`/`Real` widens to
/// `Double`; narrowing `Integer` to `Real` is not allowed. Every
/// `BigInt`/floating-point pair is rejected because the floating-point type
/// cannot represent every `BigInt` value.
pub const NUMERIC_PROMOTIONS: &[NumericPromotion] = &[
    NumericPromotion {
        left: DataType::SmallInt,
        right: DataType::SmallInt,
        output: DataType::SmallInt,
    },
    NumericPromotion {
        left: DataType::SmallInt,
        right: DataType::Integer,
        output: DataType::Integer,
    },
    NumericPromotion {
        left: DataType::Integer,
        right: DataType::SmallInt,
        output: DataType::Integer,
    },
    NumericPromotion {
        left: DataType::SmallInt,
        right: DataType::BigInt,
        output: DataType::BigInt,
    },
    NumericPromotion {
        left: DataType::BigInt,
        right: DataType::SmallInt,
        output: DataType::BigInt,
    },
    NumericPromotion {
        left: DataType::SmallInt,
        right: DataType::Real,
        output: DataType::Real,
    },
    NumericPromotion {
        left: DataType::Real,
        right: DataType::SmallInt,
        output: DataType::Real,
    },
    NumericPromotion {
        left: DataType::SmallInt,
        right: DataType::Double,
        output: DataType::Double,
    },
    NumericPromotion {
        left: DataType::Double,
        right: DataType::SmallInt,
        output: DataType::Double,
    },
    NumericPromotion {
        left: DataType::Integer,
        right: DataType::Integer,
        output: DataType::Integer,
    },
    NumericPromotion {
        left: DataType::Integer,
        right: DataType::BigInt,
        output: DataType::BigInt,
    },
    NumericPromotion {
        left: DataType::BigInt,
        right: DataType::Integer,
        output: DataType::BigInt,
    },
    NumericPromotion {
        left: DataType::Integer,
        right: DataType::Double,
        output: DataType::Double,
    },
    NumericPromotion {
        left: DataType::Integer,
        right: DataType::Real,
        output: DataType::Double,
    },
    NumericPromotion {
        left: DataType::Real,
        right: DataType::Integer,
        output: DataType::Double,
    },
    NumericPromotion {
        left: DataType::Double,
        right: DataType::Integer,
        output: DataType::Double,
    },
    NumericPromotion {
        left: DataType::BigInt,
        right: DataType::BigInt,
        output: DataType::BigInt,
    },
    NumericPromotion {
        left: DataType::Real,
        right: DataType::Real,
        output: DataType::Real,
    },
    NumericPromotion {
        left: DataType::Real,
        right: DataType::Double,
        output: DataType::Double,
    },
    NumericPromotion {
        left: DataType::Double,
        right: DataType::Real,
        output: DataType::Double,
    },
    NumericPromotion {
        left: DataType::Double,
        right: DataType::Double,
        output: DataType::Double,
    },
];

pub fn promote_numeric(left: DataType, right: DataType) -> Option<DataType> {
    NUMERIC_PROMOTIONS
        .iter()
        .find(|entry| entry.left == left && entry.right == right)
        .map(|entry| entry.output)
}
