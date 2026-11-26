// SPDX-License-Identifier: GPL-2.0-or-later
// Contains code ported line for line from R 4.4.2's src/appl/integrate.c (rdqagse, rdqagie),
// which is GPL-2.0-or-later (D279).

//! RFC-0019 §S6: QUADPACK's QAGS and QAGI, as R's `integrate` runs them.
//!
//! A port of R 4.4.2's `src/appl/integrate.c`: `rdqagse` and `rdqagie`, the
//! one adaptive driver they share (bisect the subinterval of largest error,
//! keep the error list ordered with `rdqpsrt`, extrapolate with Wynn's
//! ε-algorithm, `rdqelg`), over `rdqk21`'s 21-point Gauss–Kronrod rule on a
//! finite range and `rdqk15i`'s 15-point rule on an infinite one mapped to
//! (0, 1] by x = bound ± (1 − t)/t. The two drivers differ only in the rule,
//! the first interval, and how `ier`'s codes above 2 are numbered; Bund2
//! asks only whether `ier` is 0, so one driver serves both.
//!
//! The integrand is a closure that may fail, so a lambda's refusal (E112)
//! or error passes straight out. Indices are QUADPACK's, from 1; slot 0 of
//! each list is unused, and each list grows by one slot per subdivision, so
//! a large `limit` costs nothing until it is used.
//!
//! The rules' weights and abscissae are `integrate.c`'s, digit for digit;
//! they carry more digits than an f64 holds, and rounding them by hand
//! would only invite a slip.
#![allow(clippy::excessive_precision)]

/// An integral, its error estimate, the subintervals used, and QUADPACK's
/// `ier`: 0 when the requested accuracy was reached.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Quad {
    pub(crate) value: f64,
    pub(crate) abs_error: f64,
    pub(crate) subdivisions: usize,
    pub(crate) ier: u8,
}

/// The range: finite ends, or QAGI's `bound` and `inf` (1 for
/// (bound, ∞), −1 for (−∞, bound), 2 for (−∞, ∞) with `bound` 0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Range {
    Finite(f64, f64),
    Infinite { bound: f64, inf: i8 },
}

/// One rule's answer on (a, b): the integral, its error, ∫|f| and
/// ∫|f − mean|, QUADPACK's `result`, `abserr`, `resabs` and `resasc`.
type Rule = (f64, f64, f64, f64);

const EPMACH: f64 = f64::EPSILON;
const UFLOW: f64 = f64::MIN_POSITIVE;
const OFLOW: f64 = f64::MAX;

/// `rdqk21`'s Gauss weights, Kronrod abscissae and Kronrod weights.
const WG21: [f64; 5] = [
    0.066_671_344_308_688_137_593_568_809_893_332,
    0.149_451_349_150_580_593_145_776_339_657_697,
    0.219_086_362_515_982_043_995_534_934_228_163,
    0.269_266_719_309_996_355_091_226_921_569_469,
    0.295_524_224_714_752_870_173_892_994_651_338,
];
const XGK21: [f64; 11] = [
    0.995_657_163_025_808_080_735_527_280_689_003,
    0.973_906_528_517_171_720_077_964_012_084_452,
    0.930_157_491_355_708_226_001_207_180_059_508,
    0.865_063_366_688_984_510_732_096_688_423_493,
    0.780_817_726_586_416_897_063_717_578_345_042,
    0.679_409_568_299_024_406_234_327_365_114_874,
    0.562_757_134_668_604_683_339_000_099_272_694,
    0.433_395_394_129_247_190_799_265_943_165_784,
    0.294_392_862_701_460_198_131_126_603_103_866,
    0.148_874_338_981_631_210_884_826_001_129_72,
    0.0,
];
const WGK21: [f64; 11] = [
    0.011_694_638_867_371_874_278_064_396_062_192,
    0.032_558_162_307_964_727_478_818_972_459_39,
    0.054_755_896_574_351_996_031_381_300_244_58,
    0.075_039_674_810_919_952_767_043_140_916_19,
    0.093_125_454_583_697_605_535_065_465_083_366,
    0.109_387_158_802_297_641_899_210_590_325_805,
    0.123_491_976_262_065_851_077_958_109_831_074,
    0.134_709_217_311_473_325_928_054_001_771_707,
    0.142_775_938_577_060_080_797_094_273_138_717,
    0.147_739_104_901_338_491_374_841_515_972_068,
    0.149_445_554_002_916_905_664_936_468_389_821,
];

/// `rdqk15i`'s Gauss weights, Kronrod abscissae and Kronrod weights.
const WG15: [f64; 8] = [
    0.0,
    0.129_484_966_168_869_693_270_611_432_679_082,
    0.0,
    0.279_705_391_489_276_667_901_467_771_423_78,
    0.0,
    0.381_830_050_505_118_944_950_369_775_488_975,
    0.0,
    0.417_959_183_673_469_387_755_102_040_816_327,
];
const XGK15: [f64; 8] = [
    0.991_455_371_120_812_639_206_854_697_526_329,
    0.949_107_912_342_758_524_526_189_684_047_851,
    0.864_864_423_359_769_072_789_712_788_640_926,
    0.741_531_185_599_394_439_863_864_773_280_788,
    0.586_087_235_467_691_130_294_144_838_258_73,
    0.405_845_151_377_397_166_906_606_412_076_961,
    0.207_784_955_007_898_467_600_689_403_773_245,
    0.0,
];
const WGK15: [f64; 8] = [
    0.022_935_322_010_529_224_963_732_008_058_97,
    0.063_092_092_629_978_553_290_700_663_189_204,
    0.104_790_010_322_250_183_839_876_322_541_518,
    0.140_653_259_715_525_918_745_189_590_510_238,
    0.169_004_726_639_267_902_826_583_426_598_55,
    0.190_350_578_064_785_409_913_256_402_421_014,
    0.204_432_940_075_298_892_414_161_999_234_649,
    0.209_482_141_084_727_828_012_999_174_891_714,
];

/// The error estimate both rules finish with.
fn rule_error(resk: f64, resg: f64, hlgth: f64, resabs: f64, resasc: f64) -> f64 {
    let mut abserr = ((resk - resg) * hlgth).abs();
    if resasc != 0.0 && abserr != 0.0 {
        abserr = resasc * (abserr * 200.0 / resasc).powf(1.5).min(1.0);
    }
    if resabs > UFLOW / (EPMACH * 50.0) {
        abserr = abserr.max(EPMACH * 50.0 * resabs);
    }
    abserr
}

/// `rdqk21`: the 21-point Kronrod rule on (a, b), f asked in R's order.
fn qk21<E>(f: &mut dyn FnMut(f64) -> Result<f64, E>, a: f64, b: f64) -> Result<Rule, E> {
    let centr = (a + b) * 0.5;
    let hlgth = (b - a) * 0.5;
    let dhlgth = hlgth.abs();
    let mut v = [0.0; 21];
    v[0] = f(centr)?;
    for j in 1..=5 {
        let absc = hlgth * XGK21[2 * j - 1];
        v[2 * j - 1] = f(centr - absc)?;
        v[2 * j] = f(centr + absc)?;
    }
    for j in 1..=5 {
        let absc = hlgth * XGK21[2 * j - 2];
        v[2 * j + 9] = f(centr - absc)?;
        v[2 * j + 10] = f(centr + absc)?;
    }
    let fc = v[0];
    let mut resg = 0.0;
    let mut resk = WGK21[10] * fc;
    let mut resabs = resk.abs();
    let (mut fv1, mut fv2) = ([0.0; 10], [0.0; 10]);
    for j in 1..=5 {
        let jtw = 2 * j;
        let (f1, f2) = (v[2 * j - 1], v[2 * j]);
        fv1[jtw - 1] = f1;
        fv2[jtw - 1] = f2;
        let fsum = f1 + f2;
        resg += WG21[j - 1] * fsum;
        resk += WGK21[jtw - 1] * fsum;
        resabs += WGK21[jtw - 1] * (f1.abs() + f2.abs());
    }
    for j in 1..=5 {
        let jtwm1 = 2 * j - 1;
        let (f1, f2) = (v[2 * j + 9], v[2 * j + 10]);
        fv1[jtwm1 - 1] = f1;
        fv2[jtwm1 - 1] = f2;
        let fsum = f1 + f2;
        resk += WGK21[jtwm1 - 1] * fsum;
        resabs += WGK21[jtwm1 - 1] * (f1.abs() + f2.abs());
    }
    let reskh = resk * 0.5;
    let mut resasc = WGK21[10] * (fc - reskh).abs();
    for j in 0..10 {
        resasc += WGK21[j] * ((fv1[j] - reskh).abs() + (fv2[j] - reskh).abs());
    }
    let result = resk * hlgth;
    let resabs = resabs * dhlgth;
    let resasc = resasc * dhlgth;
    let abserr = rule_error(resk, resg, hlgth, resabs, resasc);
    Ok((result, abserr, resabs, resasc))
}

/// `rdqk15i`: the 15-point Kronrod rule on (a, b) ⊂ (0, 1], the range
/// mapped by x = bound + dinf·(1 − t)/t, both halves at once when `inf` is 2.
fn qk15i<E>(
    f: &mut dyn FnMut(f64) -> Result<f64, E>,
    boun: f64,
    inf: i8,
    a: f64,
    b: f64,
) -> Result<Rule, E> {
    let dinf = f64::from(inf.min(1));
    let centr = (a + b) * 0.5;
    let hlgth = (b - a) * 0.5;
    let map = |t: f64| boun + dinf * (1.0 - t) / t;
    let mut ts = [0.0; 15];
    ts[0] = centr;
    for j in 1..=7 {
        let absc = hlgth * XGK15[j - 1];
        ts[2 * j - 1] = centr - absc;
        ts[2 * j] = centr + absc;
    }
    let mut v = [0.0; 15];
    for (y, t) in v.iter_mut().zip(&ts) {
        *y = f(map(*t))?;
    }
    if inf == 2 {
        for (y, t) in v.iter_mut().zip(&ts) {
            *y += f(-map(*t))?;
        }
    }
    let fc = v[0] / centr / centr;
    let mut resg = WG15[7] * fc;
    let mut resk = WGK15[7] * fc;
    let mut resabs = resk.abs();
    let (mut fv1, mut fv2) = ([0.0; 7], [0.0; 7]);
    for j in 1..=7 {
        let (t1, t2) = (ts[2 * j - 1], ts[2 * j]);
        let f1 = v[2 * j - 1] / t1 / t1;
        let f2 = v[2 * j] / t2 / t2;
        fv1[j - 1] = f1;
        fv2[j - 1] = f2;
        let fsum = f1 + f2;
        resg += WG15[j - 1] * fsum;
        resk += WGK15[j - 1] * fsum;
        resabs += WGK15[j - 1] * (f1.abs() + f2.abs());
    }
    let reskh = resk * 0.5;
    let mut resasc = WGK15[7] * (fc - reskh).abs();
    for j in 0..7 {
        resasc += WGK15[j] * ((fv1[j] - reskh).abs() + (fv2[j] - reskh).abs());
    }
    let result = resk * hlgth;
    let resasc = resasc * hlgth;
    let resabs = resabs * hlgth;
    let abserr = rule_error(resk, resg, hlgth, resabs, resasc);
    Ok((result, abserr, resabs, resasc))
}

/// ∫ f over `range` to max(epsabs, epsrel·|∫f|), at most `limit`
/// subintervals: QAGS on a finite range, QAGI on an infinite one.
pub(crate) fn integrate<E>(
    f: &mut dyn FnMut(f64) -> Result<f64, E>,
    range: Range,
    epsabs: f64,
    epsrel: f64,
    limit: usize,
) -> Result<Quad, E> {
    match range {
        Range::Finite(a, b) => adaptive(&mut |x, y| qk21(f, x, y), a, b, epsabs, epsrel, limit),
        Range::Infinite { bound, inf } => {
            let boun = if inf == 2 { 0.0 } else { bound };
            adaptive(
                &mut |x, y| qk15i(f, boun, inf, x, y),
                0.0,
                1.0,
                epsabs,
                epsrel,
                limit,
            )
        }
    }
}

/// `rdqagse`'s and `rdqagie`'s driver over `rule` on (a, b).
#[allow(clippy::too_many_lines)]
fn adaptive<E>(
    rule: &mut dyn FnMut(f64, f64) -> Result<Rule, E>,
    a: f64,
    b: f64,
    epsabs: f64,
    epsrel: f64,
    limit: usize,
) -> Result<Quad, E> {
    let done = |value: f64, abs_error: f64, subdivisions: usize, ier: u8| Quad {
        value,
        abs_error,
        subdivisions,
        ier,
    };
    if limit < 1 || (epsabs <= 0.0 && epsrel < (EPMACH * 50.0).max(5e-29)) {
        return Ok(done(0.0, 0.0, 0, 6));
    }
    let (mut result, mut abserr, defabs, resabs) = rule(a, b)?;
    let dres = result.abs();
    let mut errbnd = epsabs.max(epsrel * dres);
    let mut ier = 0u8;
    if abserr <= EPMACH * 100.0 * defabs && abserr > errbnd {
        ier = 2;
    }
    if limit == 1 {
        ier = 1;
    }
    if ier != 0 || (abserr <= errbnd && abserr != resabs) || abserr == 0.0 {
        return Ok(done(result, abserr, 1, ier));
    }

    let mut alist = vec![0.0, a];
    let mut blist = vec![0.0, b];
    let mut rlist = vec![0.0, result];
    let mut elist = vec![0.0, abserr];
    let mut iord = vec![0usize, 1];
    let mut rlist2 = [0.0; 53];
    let mut res3la = [0.0; 4];
    rlist2[1] = result;
    let mut errmax = abserr;
    let mut maxerr = 1usize;
    let mut area = result;
    let mut errsum = abserr;
    abserr = OFLOW;
    let mut nrmax = 1usize;
    let mut nres = 0usize;
    let mut numrl2 = 2usize;
    let mut ktmin = 0u32;
    let mut extrap = false;
    let mut noext = false;
    let mut ierro = 0u8;
    let (mut iroff1, mut iroff2, mut iroff3) = (0u32, 0u32, 0u32);
    let ksgn: i8 = if dres >= (1.0 - EPMACH * 50.0) * defabs {
        1
    } else {
        -1
    };
    let (mut small, mut erlarg, mut ertest, mut correc) = (0.0, 0.0, 0.0, 0.0);
    let mut last = 1usize;
    let mut sum_all = false;

    for l in 2..=limit {
        last = l;
        alist.push(0.0);
        blist.push(0.0);
        rlist.push(0.0);
        elist.push(0.0);
        iord.push(0);

        let a1 = alist[maxerr];
        let b1 = (alist[maxerr] + blist[maxerr]) * 0.5;
        let a2 = b1;
        let b2 = blist[maxerr];
        let erlast = errmax;
        let (area1, error1, _, defab1) = rule(a1, b1)?;
        let (area2, error2, _, defab2) = rule(a2, b2)?;
        let area12 = area1 + area2;
        let erro12 = error1 + error2;
        errsum = errsum + erro12 - errmax;
        area = area + area12 - rlist[maxerr];
        if !(defab1 == error1 || defab2 == error2) {
            if (rlist[maxerr] - area12).abs() <= area12.abs() * 1e-5 && erro12 >= errmax * 0.99 {
                if extrap {
                    iroff2 += 1;
                } else {
                    iroff1 += 1;
                }
            }
            if last > 10 && erro12 > errmax {
                iroff3 += 1;
            }
        }
        rlist[maxerr] = area1;
        rlist[last] = area2;
        errbnd = epsabs.max(epsrel * area.abs());
        if iroff1 + iroff2 >= 10 || iroff3 >= 20 {
            ier = 2;
        }
        if iroff2 >= 5 {
            ierro = 3;
        }
        if last == limit {
            ier = 1;
        }
        if a1.abs().max(b2.abs()) <= (EPMACH * 100.0 + 1.0) * (a2.abs() + UFLOW * 1e3) {
            ier = 4;
        }
        if error2 > error1 {
            alist[maxerr] = a2;
            alist[last] = a1;
            blist[last] = b1;
            rlist[maxerr] = area2;
            rlist[last] = area1;
            elist[maxerr] = error2;
            elist[last] = error1;
        } else {
            alist[last] = a2;
            blist[maxerr] = b1;
            blist[last] = b2;
            elist[maxerr] = error1;
            elist[last] = error2;
        }
        qpsrt(
            limit,
            last,
            &mut maxerr,
            &mut errmax,
            &elist,
            &mut iord,
            &mut nrmax,
        );
        if errsum <= errbnd {
            sum_all = true;
            break;
        }
        if ier != 0 {
            break;
        }
        if last == 2 {
            small = (b - a).abs() * 0.375;
            erlarg = errsum;
            ertest = errbnd;
            rlist2[2] = area;
            continue;
        }
        if noext {
            continue;
        }
        erlarg -= erlast;
        if (b1 - a1).abs() > small {
            erlarg += erro12;
        }
        if !extrap {
            if (blist[maxerr] - alist[maxerr]).abs() > small {
                continue;
            }
            extrap = true;
            nrmax = 2;
        }
        if ierro != 3 && erlarg > ertest {
            let id = nrmax;
            let jupbnd = if last > limit / 2 + 2 {
                limit + 3 - last
            } else {
                last
            };
            let mut larger = false;
            for _ in id..=jupbnd {
                maxerr = iord[nrmax];
                errmax = elist[maxerr];
                if (blist[maxerr] - alist[maxerr]).abs() > small {
                    larger = true;
                    break;
                }
                nrmax += 1;
            }
            if larger {
                continue;
            }
        }
        numrl2 += 1;
        rlist2[numrl2] = area;
        let (reseps, abseps) = qelg(&mut numrl2, &mut rlist2, &mut res3la, &mut nres);
        ktmin += 1;
        if ktmin > 5 && abserr < errsum * 0.001 {
            ier = 5;
        }
        if abseps < abserr {
            ktmin = 0;
            abserr = abseps;
            result = reseps;
            correc = erlarg;
            ertest = epsabs.max(epsrel * reseps.abs());
            if abserr <= ertest {
                break;
            }
        }
        if numrl2 == 1 {
            noext = true;
        }
        if ier == 5 {
            break;
        }
        maxerr = iord[1];
        errmax = elist[maxerr];
        nrmax = 1;
        extrap = false;
        small *= 0.5;
        erlarg = errsum;
    }

    // L100: the final result and error estimate.
    if !sum_all && abserr != OFLOW {
        let mut test = true;
        if ier + ierro != 0 {
            if ierro == 3 {
                abserr += correc;
            }
            if ier == 0 {
                ier = 3;
            }
            if result == 0.0 || area == 0.0 {
                if abserr > errsum {
                    sum_all = true;
                } else if area == 0.0 {
                    test = false;
                }
            } else if abserr / result.abs() > errsum / area.abs() {
                sum_all = true;
            }
        }
        // L110: the test on divergence.
        if !sum_all
            && test
            && !(ksgn == -1 && result.abs().max(area.abs()) <= defabs * 0.01)
            && (0.01 > result / area || result / area > 100.0 || errsum > area.abs())
        {
            ier = 5;
        }
    } else {
        sum_all = true;
    }
    if sum_all {
        // L115: the sum over the subintervals.
        result = rlist[1..=last].iter().sum();
        abserr = errsum;
    }
    Ok(done(result, abserr, last, ier))
}

/// `rdqpsrt`: keep `iord` ordered by descending error, and pick the
/// subinterval with the `nrmax`-th largest to bisect next.
fn qpsrt(
    limit: usize,
    last: usize,
    maxerr: &mut usize,
    ermax: &mut f64,
    elist: &[f64],
    iord: &mut [usize],
    nrmax: &mut usize,
) {
    if last <= 2 {
        iord[1] = 1;
        iord[2] = 2;
    } else {
        let errmax = elist[*maxerr];
        if *nrmax > 1 {
            for _ in 1..*nrmax {
                let isucc = iord[*nrmax - 1];
                if errmax <= elist[isucc] {
                    break;
                }
                iord[*nrmax] = isucc;
                *nrmax -= 1;
            }
        }
        let jupbn = if last > limit / 2 + 2 {
            limit + 3 - last
        } else {
            last
        };
        let errmin = elist[last];
        let jbnd = jupbn - 1;
        let mut placed = false;
        let mut i = *nrmax + 1;
        while i <= jbnd {
            let isucc = iord[i];
            if errmax >= elist[isucc] {
                // L60: insert errmin by traversing the list bottom-up.
                iord[i - 1] = *maxerr;
                let mut k = jbnd;
                let mut inserted = false;
                for _ in i..=jbnd {
                    let isucc = iord[k];
                    if errmin < elist[isucc] {
                        iord[k + 1] = last;
                        inserted = true;
                        break;
                    }
                    iord[k + 1] = isucc;
                    k -= 1;
                }
                if !inserted {
                    iord[i] = last;
                }
                placed = true;
                break;
            }
            iord[i - 1] = isucc;
            i += 1;
        }
        if !placed {
            iord[jbnd] = *maxerr;
            iord[jupbn] = last;
        }
    }
    *maxerr = iord[*nrmax];
    *ermax = elist[*maxerr];
}

/// `rdqelg`: Wynn's ε-algorithm over the table's `n` entries, from 1.
/// Answers the extrapolated limit and its error estimate.
fn qelg(
    n: &mut usize,
    epstab: &mut [f64; 53],
    res3la: &mut [f64; 4],
    nres: &mut usize,
) -> (f64, f64) {
    const LIMEXP: usize = 50;
    *nres += 1;
    let mut abserr = OFLOW;
    let mut result = epstab[*n];
    let finish = |result: f64, abserr: f64| (result, abserr.max(EPMACH * 5.0 * result.abs()));
    if *n < 3 {
        return finish(result, abserr);
    }
    epstab[*n + 2] = epstab[*n];
    let newelm = (*n - 1) / 2;
    epstab[*n] = OFLOW;
    let num = *n;
    let mut k1 = *n;
    for i in 1..=newelm {
        let k2 = k1 - 1;
        let k3 = k1 - 2;
        let res = epstab[k1 + 2];
        let e0 = epstab[k3];
        let e1 = epstab[k2];
        let e2 = res;
        let e1abs = e1.abs();
        let delta2 = e2 - e1;
        let err2 = delta2.abs();
        let tol2 = e2.abs().max(e1abs) * EPMACH;
        let delta3 = e1 - e0;
        let err3 = delta3.abs();
        let tol3 = e1abs.max(e0.abs()) * EPMACH;
        if err2 <= tol2 && err3 <= tol3 {
            // e0, e1 and e2 agree to machine accuracy: converged.
            return finish(res, err2 + err3);
        }
        let e3 = epstab[k1];
        epstab[k1] = e1;
        let delta1 = e1 - e3;
        let err1 = delta1.abs();
        let tol1 = e1abs.max(e3.abs()) * EPMACH;
        let mut ss = 0.0;
        let mut regular = false;
        if err1 > tol1 && err2 > tol2 && err3 > tol3 {
            ss = 1.0 / delta1 + 1.0 / delta2 - 1.0 / delta3;
            regular = (ss * e1).abs() > 1e-4;
        }
        if !regular {
            *n = i + i - 1;
            break;
        }
        // L30: a new element, and perhaps a better result.
        let res = e1 + 1.0 / ss;
        epstab[k1] = res;
        k1 -= 2;
        let erra = err2 + (res - e2).abs() + err3;
        if erra <= abserr {
            abserr = erra;
            result = res;
        }
    }
    // L50: shift the table.
    if *n == LIMEXP {
        *n = 2 * (LIMEXP / 2) - 1;
    }
    let mut ib = if num.is_multiple_of(2) { 2 } else { 1 };
    for _ in 1..=newelm + 1 {
        let ib2 = ib + 2;
        epstab[ib] = epstab[ib2];
        ib = ib2;
    }
    if num != *n {
        // Move entries num − n + 1 ..= num down to 1 ..= n.
        let from = num - *n + 1;
        epstab.copy_within(from..=num, 1);
    }
    if *nres >= 4 {
        abserr =
            (result - res3la[3]).abs() + (result - res3la[2]).abs() + (result - res3la[1]).abs();
        res3la[1] = res3la[2];
        res3la[2] = res3la[3];
        res3la[3] = result;
    } else {
        res3la[*nres] = result;
        abserr = OFLOW;
    }
    finish(result, abserr)
}
