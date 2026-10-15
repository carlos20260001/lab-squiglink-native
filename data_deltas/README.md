# Delta targets

A **delta target** corrects for a measurement rig that is not the one a
published target was measured on. A 711 clone, or a substitute capsule
standing in for a B&K 5128, adds its own response to everything it measures.
Compare that reading against a target measured on the original hardware
without correcting, and the rig's error is charged to the headphone.

A delta target is that error in isolation — the clone rig's calibration minus
the original rig's — and subtracting it from a measurement puts the headphone
back in the frame every published target lives in.

Delta targets are **optional**. You can read any target without one.

## Layout

This directory is your own data, and it is deliberately not in the repository.
A calibration measured on a clone rig belongs to whoever measured it, so
everything here is ignored except this file and the example manifest:

| File | Purpose |
| --- | --- |
| `delta_targets.json` | The manifest. Names each delta target and the two calibrations that produce it. Ignored. |
| `<name>.txt` | A calibration curve: whitespace-separated or comma-separated `hz` and `dB` columns, as in `data/`. Ignored. |
| `delta_targets.example.json` | A copy of the manifest to start from. Tracked. |

To get started, copy the example and add your own:

```sh
cp delta_targets.example.json delta_targets.json
```

A delta target is defined by naming its two calibrations rather than stored as
a ready-made difference, so the correction it applies is always traceable back
to the measurements behind it.

## Adding a real one

1. Measure your rig's calibration and save it as a `.txt` curve here.
2. Save the original rig's calibration the same way.
3. Add an entry to the manifest:

```json
{
  "name": "Δ 5128DF",
  "clone": "5128DF calibration.txt",
  "original": "Reference 5128 calibration.txt",
  "synthetic": false,
  "note": "Calibrated 2026-01-14 against <source>, 1/8 oct smoothed."
}
```

Both calibrations are read at the same frequencies and subtracted, so it does
not matter how densely each was measured. Outside a calibration's measured
range its edge value is held rather than extrapolated: a rig calibrated to
2 kHz says nothing about 10 kHz.

## Placeholders

A delta target can be flagged `synthetic`, and the interface carries that
through: a placeholder is labelled as one in the library, in the manager, and
wherever else it appears, because a correction nobody distrusted is worse than
no correction at all. Nothing is shipped that relies on it — an entry whose
calibrations are analytic rather than measured corrects nothing, and a stub
that looks real is the failure this flag exists to prevent.
<div class="welcome-copy"><p>Did you know that the same earphone measured by different Squiglink sites can (and usually does) result in different graphs? This is because the hardware microphones (711 couplers) we use are surprisingly variable. The 5128Δ targets were made to address this variation.</p><p>Each 5128Δ-ready squigsite is loaded with two companion targets: <b>Δ 10dB</b> and <b>IEF Comp</b>. These targets are calibrated to address the variation in couplers used by each squigsite.</p><p>Want to know more? Here's an explanation from <b>listener</b>, who did the research necessary to create the 5128Δ targets.</p><p><hr></p><p>The 5128Δ target project is a two-pronged effort that aims to make measurements between squigsites more comparable with each other, as well as more comparable with the new Brüel &amp; Kjær 5128 system measurements.</p><p><b>The first prong</b> is compensating as many squigsite couplers as possible to Crinacle's 711 coupler, which is used to create <a target="_blank" href="https://crinacle.com/graphs/iems/graphtool/">the world's largest individual database of IEM measurements</a>. Compensation is derived by averaging the differences between multiple IEM measurements that Crinacle and the target squigsites have in common.</p><p>If a user compensates to the <b>IEF Comp</b> target found in the 5128Δ category, the measurements in that database will be significantly more comparable with Crinacle's 711 measurements. Of course, it also means that the measurements can be more accurately compared with measurements on other 5128Δ-ready Squiglink sites.</p><p><b>The second prong</b> is a benefit of using Crinacle's 711 clone coupler as the basis for the aforementioned compensation.</p><p>Since I have already done analysis of the difference between Crinacle's coupler and the new Brüel &amp; Kjær 5128 measurement system (which lead to IEF's new 2023 neutral target), I can extrapolate that difference data to the other couplers that are now being compensated to Crinacle's 711 clone.</p><p>This is applied via the <b>Δ 10dB</b> target, which has the inverse of the <b>IEF Comp</b> built in. This means that if you compensate to the <b>Δ 10dB</b> target, you are theoretically getting something quite close -- at least below ~4-5kHz -- to a 5128 IEM measurement compensated to the "5128 Diffuse Field -10dB tilt" target that Crinacle and Headphones.com are using. Additionally, this makes 711 IEM measurements more comparable to headphones measured on the B&amp;K 5128 system, as headphones and IEMs no longer need different targets on the B&amp;K 5128.</p><p>If you decide to use this target without compensation, you are still getting the benefit of a target that sounds quite similar to the -10dB 5128 Diffuse Field tilt, but you may notice it looks different on each squigsite. That's because each target is compensated for each squigsite's coupler idiosyncrasies.</p><p>The goals here are to make measurements across various sites and rigs more comparable, and it achieves that goal through compensating for many coupler idiosyncrasies, as well as nudging users towards reading compensated graphs, as this is the best way to visually compare data from different sources/measurement rigs.</p><p>Any questions about the calculation of this can be directed at me (<b>listener.</b> on Discord), and any questions about the implementation can be directed at either me or MRS (<b>typeMRS</b> on Discord).</p><p>— <b>listener</b></p><ul class="slide-links-list"><li class="slide-link" data-nav="2">How to use 5128Δ targets</li></ul></div>
<div class="welcome-copy"><p><b>Step 1:</b> Select one of the 5128Δ targets (<b>Δ 10dB</b> or <b>IEF Comp</b>) to automatically compensate the graphs you view on this squigsite.</p><p><b>Step 2:</b> When you view graphs on another 5128Δ-ready squigsite, use the same target to make those graphs directly comparable with the graphs on this site.</p><p><hr></p><p>Comparing graphs between squigsites <i>without</i> compensation is likely to lead to misleading conclusions.</p><p>Even with compensation, take results with a grain of salt, but 5128Δ target compensation yields a big improvement in confidence.</p><p>Without compensation, estimate about a 40% confidence level when comparing graphs from one squigsite to another. With 5128Δ target compensation, confidence of comparisons improves to about 80%, which is enough to be useful.</p><</div>
