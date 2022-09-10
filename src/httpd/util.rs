pub fn format_data_size(size: u64) -> String {
    const ONE_KB: u64 = 1000;
    const ONE_MB: u64 = ONE_KB * ONE_KB;
    const ONE_GB: u64 = ONE_MB * ONE_KB;
    const ONE_TB: u64 = ONE_GB * ONE_KB;

    const FOUR_KB: u64 = 4 * ONE_KB;
    const FOUR_MB: u64 = 4 * ONE_MB;
    const FOUR_GB: u64 = 4 * ONE_GB;

    const U32_MAX: u64 = u32::MAX as u64;

    const MAX_0: u64 = U32_MAX;
    const MIN_1: u64 = MAX_0 + 1;
    const MAX_1: u64 = U32_MAX * ONE_KB;
    const MIN_2: u64 = MAX_1 + 1;
    const MAX_2: u64 = U32_MAX * ONE_MB;
    const MIN_3: u64 = MAX_2 + 1;
    const MAX_3: u64 = U32_MAX * ONE_GB;

    const ONE_KB_F64: f64 = 1000.0;
    const ONE_MB_F64: f64 = ONE_KB_F64 * ONE_KB_F64;
    const ONE_GB_F64: f64 = ONE_MB_F64 * ONE_KB_F64;

    const UNIT: [&str; 7] = ["B", "kB", "MB", "GB", "TB", "PB", "EB"];

    /*
    let mut range = 0;
    let mut size = size;
    while size > U32_MAX {
        size /= ONE_KB;
        range += 1;
    }
    */

    let (size, range) = match size {
        0..=MAX_0 => (size, 0),
        MIN_1..=MAX_1 => (size / ONE_KB, 1),
        MIN_2..=MAX_2 => (size / ONE_MB, 2),
        MIN_3..=MAX_3 => (size / ONE_GB, 3),
        _ => (size / ONE_TB, 4),
    };

    // size = 0..=U32_MAX, range = 0..6
    match size {
        0..=FOUR_KB => format!("{} {}", size, UNIT[range]),
        FOUR_KB..=FOUR_MB => format!(
            "{:.2} {}",
            f64::from(size as u32) / ONE_KB_F64,
            UNIT[range + 1]
        ),
        FOUR_MB..=FOUR_GB => format!(
            "{:.2} {}",
            f64::from(size as u32) / ONE_MB_F64,
            UNIT[range + 2]
        ),
        _ => format!(
            "{:.2} {}",
            f64::from(size as u32) / ONE_GB_F64,
            UNIT[range + 3]
        ),
    }
}

pub fn format_percent(fract: u64, of: u64) -> String {
    fn downsize(val: u64) -> Option<u32> {
        const U32: u64 = 1 << 32;
        const U48: u64 = 1 << 48;
        const U32_MAX: u64 = u32::MAX as u64;
        const U48_MAX: u64 = U48 - 1;

        match val {
            0..=U32_MAX => None,
            U32..=U48_MAX => Some(16),
            _ => Some(32),
        }
    }

    let (fract, of) = if fract >= of {
        if let Some(shift) = downsize(fract) {
            (fract >> shift, of >> shift)
        } else {
            (fract, of)
        }
    } else {
        if let Some(shift) = downsize(of) {
            (fract >> shift, of >> shift)
        } else {
            (fract, of)
        }
    };
    format!(
        "{:.2}",
        f64::from(fract as u32) / f64::from(of as u32) * 100.0
    )
}

#[cfg(test)]
mod tests {
    use crate::httpd::util::format_data_size;
    const U32_MAX: u64 = u32::MAX as u64;

    #[test]
    fn fds_01() {
        assert_eq!(format_data_size(2048).as_str(), "2048 B");
        assert_eq!(format_data_size(4000).as_str(), "4000 B")
    }

    #[test]
    fn fds_02() {
        assert_eq!(format_data_size(4 * 1000 + 1).as_str(), "4.00 kB");
        assert_eq!(format_data_size(6 * 1000).as_str(), "6.00 kB")
    }

    #[test]
    fn fds_03() {
        assert_eq!(format_data_size(4 * 1000000 + 1).as_str(), "4.00 MB");
        assert_eq!(format_data_size(6 * 1000000).as_str(), "6.00 MB")
    }

    #[test]
    fn fds_04() {
        assert_eq!(format_data_size(U32_MAX).as_str(), "4.29 GB");
        assert_eq!(format_data_size(4 * 1000000000 + 1).as_str(), "4.00 GB");
        assert_eq!(format_data_size(6 * 1000000000).as_str(), "6.00 GB")
    }

    #[test]
    fn fds_05() {
        assert_eq!(format_data_size(6 * 1000000000000).as_str(), "6.00 TB")
    }

    #[test]
    fn fds_06() {
        assert_eq!(format_data_size(6 * 1000000000000000).as_str(), "6.00 PB");
        assert_eq!(format_data_size(u64::MAX).as_str(), "18.45 EB")
    }
}
