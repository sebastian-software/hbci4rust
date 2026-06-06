#[derive(Debug, Clone, Copy, Default)]
pub struct AccountCrcAlgs;

impl AccountCrcAlgs {
    pub fn check_iban(iban: &str) -> bool {
        if iban.len() < 4 {
            return false;
        }

        mod97(
            iban.as_bytes()[4..]
                .iter()
                .chain(&iban.as_bytes()[..4])
                .copied(),
        )
    }

    pub fn check_creditor_id(creditor_id: &str) -> bool {
        if creditor_id.len() < 7 {
            return false;
        }

        if creditor_id.as_bytes()[..2].eq_ignore_ascii_case(b"DE") && creditor_id.len() != 18 {
            return false;
        }

        mod97(
            creditor_id.as_bytes()[7..]
                .iter()
                .chain(&creditor_id.as_bytes()[..4])
                .copied(),
        )
    }

    pub fn alg_51(_blz: Option<&[u8; 8]>, number: &[u8; 10]) -> bool {
        if number[2] != 9 {
            Self::alg_51_method_a(number)
                || Self::alg_51_method_b(number)
                || Self::alg_51_method_c(number)
                || (number[9] < 7 && Self::alg_51_method_d(number))
        } else {
            Self::alg_51_variant_1(number) || Self::alg_51_variant_2(number)
        }
    }

    fn alg_51_method_a(number: &[u8; 10]) -> bool {
        let sum = add_products(number, 3, 8, &[7, 6, 5, 4, 3, 2], false);
        let mut crc = 11 - sum % 11;
        if crc > 9 {
            crc = 0;
        }
        u32::from(number[9]) == crc
    }

    fn alg_51_method_b(number: &[u8; 10]) -> bool {
        let sum = add_products(number, 4, 8, &[6, 5, 4, 3, 2], false);
        let mut crc = 11 - sum % 11;
        if crc > 9 {
            crc = 0;
        }
        u32::from(number[9]) == crc
    }

    fn alg_51_method_c(number: &[u8; 10]) -> bool {
        let sum = add_products(number, 3, 8, &[1, 2, 1, 2, 1, 2], true);
        let crc = (10 - sum % 10) % 10;
        u32::from(number[9]) == crc
    }

    fn alg_51_method_d(number: &[u8; 10]) -> bool {
        let sum = add_products(number, 4, 8, &[6, 5, 4, 3, 2], false);
        let mut crc = (7 - sum % 7) % 7;
        if crc > 9 {
            crc = 0;
        }
        u32::from(number[9]) == crc
    }

    fn alg_51_variant_1(number: &[u8; 10]) -> bool {
        let sum = add_products(number, 2, 8, &[8, 7, 6, 5, 4, 3, 2], false);
        let mut crc = 11 - sum % 11;
        if crc > 9 {
            crc = 0;
        }
        u32::from(number[9]) == crc
    }

    fn alg_51_variant_2(number: &[u8; 10]) -> bool {
        let sum = add_products(number, 0, 8, &[10, 9, 8, 7, 6, 5, 4, 3, 2], false);
        let mut crc = 11 - sum % 11;
        if crc > 9 {
            crc = 0;
        }
        u32::from(number[9]) == crc
    }
}

fn mod97(bytes: impl IntoIterator<Item = u8>) -> bool {
    let mut remainder = 0u32;
    for byte in bytes {
        match byte {
            b'0'..=b'9' => {
                remainder = (remainder * 10 + u32::from(byte - b'0')) % 97;
            }
            b'A'..=b'Z' => {
                remainder = (remainder * 100 + u32::from(byte - b'A' + 10)) % 97;
            }
            _ => return false,
        }
    }

    remainder == 1
}

fn add_products(
    number: &[u8; 10],
    first: usize,
    last: usize,
    factors: &[u8],
    with_checksum: bool,
) -> u32 {
    let mut result = 0;
    for index in first..=last {
        let mut product = u32::from(number[index]) * u32::from(factors[index - first]);
        if with_checksum {
            product = digit_sum(product, false);
        }
        result += product;
    }
    result
}

fn digit_sum(mut value: u32, recursive: bool) -> u32 {
    let mut sum = 0;
    while value > 0 {
        sum += value % 10;
        value /= 10;
    }
    if recursive && sum >= 10 {
        sum = digit_sum(sum, true);
    }
    sum
}
