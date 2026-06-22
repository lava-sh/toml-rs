#[macro_export]
macro_rules! parse_int {
    ($int:ty, $bytes:expr, $options:expr, $radix:expr) => {
        match $radix {
            2 => lexical_core::parse_with_options::<
                $int,
                { lexical_core::NumberFormatBuilder::from_radix(2) },
            >($bytes, $options),
            8 => lexical_core::parse_with_options::<
                $int,
                { lexical_core::NumberFormatBuilder::from_radix(8) },
            >($bytes, $options),
            10 => lexical_core::parse_with_options::<
                $int,
                { lexical_core::NumberFormatBuilder::from_radix(10) },
            >($bytes, $options),
            16 => lexical_core::parse_with_options::<
                $int,
                { lexical_core::NumberFormatBuilder::from_radix(16) },
            >($bytes, $options),
            _ => unreachable!(),
        }
    };
}
