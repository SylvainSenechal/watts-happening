# Future Feature Suggestions

Potential metrics and visualizations to add to the dashboard:

1. **Normalized Power (NP)** - Accounts for variability, better than avg power for intervals
2. **Intensity Factor (IF)** - NP as % of your FTP, shows how hard each ride was
3. **Training Load (TSS)** - Combines duration + intensity, track weekly load
4. **Power zones distribution** - Time spent in Z1-Z5 per ride, see if you're training right zones
5. **Heart Rate Recovery** - How fast HR drops after hard efforts (needs more granular analysis)
6. **Fatigue Index** - Power fade in last 20% vs first 20%
7. **Cadence vs Power efficiency** - Find your optimal cadence
8. **Weekly/Monthly volume trends** - Hours, km, TSS over time
9. **Best efforts** - Track PRs (5min, 20min power)

---

## Notes

### Efficiency Factor (W/bpm)

Currently using **Average Power / Average HR** which is a simplified version.

The proper definition uses **Normalized Power (NP)** instead of average power:

> "Efficiency Factor is normalized watts (output) divided by average heart rate (input). 
> The question that the Efficiency Factor is answering is how aerobically fit, are you? 
> The Efficiency Factor should be used for tracking your aerobic level."
> — cyklopedia.cc / intervals.icu

**Why NP matters**: Normalized Power accounts for the variability of your effort. A ride with lots of surges and recoveries has a higher "cost" than a steady ride at the same average power. NP better reflects the true physiological demand.

**Note**: Performance varies substantially from day to day, so expect these numbers to be fairly noisy from ride to ride. Over a longer period, you should see a distinct upward trend in efficiency factor as your aerobic fitness increases.

### TODO: Implement Normalized Power

NP Formula:
1. Calculate 30-second rolling average of power
2. Raise each value to the 4th power
3. Take the average of those values
4. Take the 4th root of that average

```
NP = ⁴√(avg(rolling_30s_power⁴))
```
