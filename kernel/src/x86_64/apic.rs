#[init]
#[allow(clippy::missing_safety_doc)]
pub unsafe fn remap() {
    // FIXME: Disable the caching in pagination for the IO mapped registers.
    // Intel manual recommand to do a strong uncached memory mapping for the LAPIC,
    // so we may be have to deal with MTRRs and/or PATs.
    // But for now, I gonna assume that everything was properly configured by the BIOS/bootloader.
}
