use nxdk_rs::kernel::time::windows_to_unix_timestamp;
use time::error::ComponentRange;
use time::OffsetDateTime;

pub fn date_from_lohi(lo: u32, hi: u32) -> Result<OffsetDateTime, ComponentRange> {
    OffsetDateTime::from_unix_timestamp(
        windows_to_unix_timestamp(
            &(((hi as u64) << 32) | (lo as u64))
        ) as i64
    )
}