#![no_std]

/// Ported from z_tmcvt inline C code.
#[inline(always)]
pub fn z_tmcvt(
    mut t: u64,
    from_hz: u32,
    to_hz: u32,
    const_hz: bool,
    round_up: bool,
    round_off: bool,
) -> u64 {
    let mul_ratio = const_hz && (to_hz > from_hz) && ((to_hz % from_hz) == 0);
    let div_ratio = const_hz && (from_hz > to_hz) && ((from_hz % to_hz) == 0);

    if from_hz == to_hz {
        return t;
    }

    let mut off: u64 = 0;

    if !mul_ratio {
        let rdivisor: u32 = if div_ratio { from_hz / to_hz } else { from_hz };
        let rdivisor: u64 = rdivisor.into();

        if round_up {
            off = rdivisor - 1;
        } else if round_off {
            off = rdivisor / 2;
        }
    }

    /* Select (at build time!) between three different expressions for
     * the same mathematical relationship, each expressed with and
     * without truncation to 32 bits (I couldn't find a way to make
     * the compiler correctly guess at the 32 bit result otherwise).
     */
    if div_ratio {
        t += off;
        t / u64::from(from_hz / to_hz)
    } else if mul_ratio {
        t * u64::from(to_hz / from_hz)
    } else {
        (t * u64::from(to_hz) + off) / u64::from(from_hz)
    }
}

#[test]
fn test_z_tmcvt() {
    let hz_ms = 1000;
    let hz_s = 1;
    let hz_ns = 1_000_000_000;

    // 500ms to 100HZ ticks
    let hz_ticks = 100;
    assert_eq!(z_tmcvt(500, hz_ms, hz_ticks, true, true, false), 50);

    // 500ms to 1000HZ ticks
    let hz_ticks = 1000;
    assert_eq!(z_tmcvt(500, hz_ms, hz_ticks, true, true, false), 500);

    // 2.5s in ns to 1000HZ ticks
    assert_eq!(z_tmcvt(2_500_000_000, hz_ns, hz_ticks, true, true, false), 2500);

    // 2500 1000HZ ticks to ns
    assert_eq!(z_tmcvt(2500, hz_ticks, hz_ns, true, true, false), 2_500_000_000);
}
