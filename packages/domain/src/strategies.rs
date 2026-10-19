use serde::{Deserialize, Serialize};

pub const RULES: &str = "committee-1.0.0";
pub const WARMUP: usize = 60;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candle {
    pub time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signal {
    pub strategy: String,
    pub score: f64,
    pub reason: String,
    pub value: f64,
    pub version: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Decision {
    pub action: String,
    pub score: f64,
    pub signals: Vec<Signal>,
    pub rules_version: String,
    pub reason: String,
}
pub fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}
pub fn ema(values: &[f64], period: usize) -> Vec<f64> {
    if values.is_empty() {
        return vec![];
    }
    let alpha = 2.0 / (period as f64 + 1.0);
    let mut result = vec![values[0]];
    for value in &values[1..] {
        result.push(alpha * value + (1.0 - alpha) * result.last().unwrap());
    }
    result
}
pub fn rsi(values: &[f64], period: usize) -> f64 {
    if values.len() <= period {
        return 50.0;
    }
    let changes: Vec<f64> = values.windows(2).map(|p| p[1] - p[0]).collect();
    let mut gain = changes[..period].iter().map(|v| v.max(0.0)).sum::<f64>() / period as f64;
    let mut loss = changes[..period].iter().map(|v| (-v).max(0.0)).sum::<f64>() / period as f64;
    for change in &changes[period..] {
        gain = (gain * (period - 1) as f64 + change.max(0.0)) / period as f64;
        loss = (loss * (period - 1) as f64 + (-change).max(0.0)) / period as f64;
    }
    if loss == 0.0 {
        if gain == 0.0 {
            50.0
        } else {
            100.0
        }
    } else {
        100.0 - 100.0 / (1.0 + gain / loss)
    }
}
fn signal(name: &str, score: f64, value: f64, reason: String) -> Signal {
    Signal {
        strategy: name.into(),
        score: score.clamp(-1.0, 1.0),
        reason,
        value,
        version: "1.0.0".into(),
    }
}
pub fn analyze(candles: &[Candle]) -> Decision {
    if candles.len() < WARMUP
        || candles
            .iter()
            .any(|c| !c.close.is_finite() || c.close <= 0.0)
        || candles.windows(2).any(|c| c[1].time - c[0].time != 60_000)
    {
        return Decision {
            action: "HOLD".into(),
            score: 0.0,
            signals: vec![],
            rules_version: RULES.into(),
            reason: "Warmup insuficiente ou série inválida/com gaps".into(),
        };
    }
    let close: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let n = close.len();
    let last = close[n - 1];
    let fast = mean(&close[n - 10..]);
    let slow = mean(&close[n - 30..]);
    let trend = (fast / slow - 1.0) * 200.0;
    let rv = rsi(&close, 14);
    let rscore = if rv < 30.0 {
        (30.0 - rv) / 15.0
    } else if rv > 70.0 {
        -(rv - 70.0) / 15.0
    } else {
        0.0
    };
    let e12 = ema(&close, 12);
    let e26 = ema(&close, 26);
    let macd: Vec<f64> = e12.iter().zip(e26.iter()).map(|(a, b)| a - b).collect();
    let histogram = macd[n - 1] - ema(&macd, 9)[n - 1];
    let middle = mean(&close[n - 20..]);
    let std = (close[n - 20..]
        .iter()
        .map(|v| (v - middle).powi(2))
        .sum::<f64>()
        / 20.0)
        .sqrt();
    let z = if std > 0.0 {
        (last - middle) / std
    } else {
        0.0
    };
    let bscore = if z.abs() > 2.0 { -z.signum() } else { 0.0 };
    let momentum = (last / close[n - 11] - 1.0) * 100.0;
    let previous = &candles[n - 21..n - 1];
    let high = previous
        .iter()
        .map(|c| c.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let low = previous.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
    let vmean = previous.iter().map(|c| c.volume).sum::<f64>() / 20.0;
    let vr = if vmean > 0.0 {
        candles[n - 1].volume / vmean
    } else {
        0.0
    };
    let breakout = if vr > 1.2 && last > high {
        1.0
    } else if vr > 1.2 && last < low {
        -1.0
    } else {
        0.0
    };
    let signals = vec![
        signal(
            "moving_average",
            trend,
            fast / slow - 1.0,
            format!("SMA10 {fast:.6} / SMA30 {slow:.6}"),
        ),
        signal(
            "rsi",
            rscore,
            rv,
            format!("RSI Wilder(14): {rv:.2}; reversão <30 / >70"),
        ),
        signal(
            "macd",
            histogram / last * 1500.0,
            histogram,
            format!("MACD 12/26/9 histograma {histogram:.8}"),
        ),
        signal(
            "bollinger",
            bscore,
            z,
            format!("Bandas 20/2; distância {z:.2} desvios"),
        ),
        signal(
            "momentum",
            momentum,
            momentum,
            format!("Retorno de 10 minutos: {momentum:.3}%"),
        ),
        signal(
            "breakout",
            breakout,
            vr,
            format!("Rompimento 20 candles, volume relativo {vr:.2}"),
        ),
    ];
    committee(signals)
}
pub fn committee(signals: Vec<Signal>) -> Decision {
    committee_with_weights(
        signals,
        &[
            ("moving_average".to_owned(), 0.30),
            ("rsi".to_owned(), 0.10),
            ("macd".to_owned(), 0.15),
            ("bollinger".to_owned(), 0.10),
            ("momentum".to_owned(), 0.25),
            ("breakout".to_owned(), 0.10),
        ]
        .into_iter()
        .collect(),
    )
}

pub fn committee_with_weights(
    signals: Vec<Signal>,
    weights: &std::collections::HashMap<String, f64>,
) -> Decision {
    let active_weight: f64 = signals
        .iter()
        .map(|s| weights.get(&s.strategy).copied().unwrap_or(0.0).clamp(0.0, 2.0))
        .sum();
    let score = if active_weight > 0.0 {
        signals.iter()
            .map(|s| s.score * weights.get(&s.strategy).copied().unwrap_or(0.0).clamp(0.0, 2.0))
            .sum::<f64>() / active_weight
    } else { 0.0 };
    let action = if score >= 0.18 {
        "BUY"
    } else if score <= -0.18 {
        "SELL"
    } else {
        "HOLD"
    };
    Decision {
        action: action.into(),
        score,
        signals,
        rules_version: RULES.into(),
        reason: format!("Consenso ponderado {score:.4}; limiares ±0.18; sem contribuição de IA"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    pub fn series() -> Vec<Candle> {
        (0..100)
            .map(|i| Candle {
                time: i * 60000,
                open: 100.0 + i as f64,
                high: 101.0 + i as f64,
                low: 99.0 + i as f64,
                close: 100.0 + i as f64,
                volume: 100.0,
            })
            .collect()
    }
    #[test]
    fn rsi_reference_extremes() {
        assert_eq!(rsi(&[1.0; 20], 14), 50.0);
        assert_eq!(
            rsi(&(1..30).map(|x| x as f64).collect::<Vec<_>>(), 14),
            100.0
        );
    }
    #[test]
    fn all_specialists_reproducible() {
        let a = analyze(&series());
        assert_eq!(a.signals.len(), 6);
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&analyze(&series())).unwrap()
        );
        assert_eq!(a.action, "BUY");
    }
    #[test]
    fn gaps_and_warmup_hold() {
        let mut data = series();
        data.remove(50);
        assert_eq!(analyze(&data).action, "HOLD");
        assert_eq!(analyze(&data[..10]).action, "HOLD");
    }
    #[test]
    fn ema_known() {
        assert_eq!(ema(&[1.0, 2.0, 3.0], 3), vec![1.0, 1.5, 2.25]);
    }
}
