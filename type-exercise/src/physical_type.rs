use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::variant_catalog::for_each_physical_family;

macro_rules! define_physical_types {
    ($( { $kind:ident, $variant:ident, $array:ident, $builder:ident, $owned:ty, $borrowed:ty } ),+ $(,)?) => {
        /// The physical representation selected at a runtime boundary.
        #[derive(Clone, Debug, Eq, Hash, PartialEq)]
        pub enum PhysicalType {
            $($variant),+,
            List(Box<PhysicalType>),
        }

        /// A catalog row used to audit the single physical-family definition.
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct PhysicalFamily {
            pub physical_type: PhysicalType,
            pub name: &'static str,
        }

        /// Every supported non-List physical family, in catalog order.
        pub const PHYSICAL_FAMILY_CATALOG: &[PhysicalFamily] = &[
            $(PhysicalFamily {
                physical_type: PhysicalType::$variant,
                name: stringify!($variant),
            }),+
        ];
    };
}

for_each_physical_family!(define_physical_types);

/// A checked erased-to-typed conversion found the wrong physical representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeMismatch {
    pub expected: PhysicalType,
    pub actual: PhysicalType,
}

impl Display for TypeMismatch {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "type mismatch: expected {:?}, got {:?}",
            self.expected, self.actual
        )
    }
}

impl Error for TypeMismatch {}
