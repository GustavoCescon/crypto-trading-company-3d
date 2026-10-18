use crate::{
    risk::{d, fill_price, quantity, RiskLimits},
    strategies::{analyze, Candle, WARMUP},
};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::Serialize;
use std::str::FromStr;

#[derive(Debug, Serialize)]
pub struct Report {
    pub simulated: bool,
    pub dataset_size: usize,
    pub split: String,
    pub trades: usize,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
    pub return_pct: f64,
    pub buy_hold_return_pct: f64,
    pub sma_reference_return_pct: f64,
    pub fees: f64,
    pub slippage_cost: f64,
    pub win_rate: f64,
    pub profit_factor: Option<f64>,
    pub expectancy: f64,
    pub max_drawdown: f64,
    pub exposure: f64,
    pub sharpe: Option<f64>,
    pub equity_curve: Vec<f64>,
    pub rules_version: String,
}
fn dec(x: f64) -> Decimal {
    Decimal::from_str(&format!("{x:.12}")).unwrap_or(Decimal::ZERO)
}
fn f(x: Decimal) -> f64 {
    x.to_f64().unwrap_or(0.0)
}
pub fn run(data: &[Candle], l: &RiskLimits, split: &str) -> Result<Report, String> {
    if data.len() < 200
        || data
            .iter()
            .any(|c| !c.open.is_finite() || c.open <= 0.0 || !c.close.is_finite() || c.close <= 0.0)
        || data.windows(2).any(|c| c[1].time - c[0].time != 60000)
    {
        return Err("São necessários >=200 candles contínuos válidos de 1m".into());
    }
    let n = data.len();
    let (start, end) = match split {
        "train" => (WARMUP, n * 6 / 10),
        "validation" => (n * 6 / 10, n * 8 / 10),
        "test" => (n * 8 / 10, n),
        _ => return Err("split deve ser train, validation ou test".into()),
    };
    let mut cash = l.initial_balance;
    let mut qty = Decimal::ZERO;
    let mut cost = Decimal::ZERO;
    let mut fees = Decimal::ZERO;
    let mut slip = Decimal::ZERO;
    let mut pnls = Vec::new();
    let mut curve = vec![f(cash)];
    let mut peak = f(cash);
    let mut dd: f64 = 0.0;
    let mut exposed = 0;
    let fee_rate = l.fee_bps / d("10000");
    for i in start..end {
        // The current open is the earliest tradable price AFTER the signal's closed candle.
        let decision = analyze(&data[i.saturating_sub(240)..i]);
        let price = dec(data[i].open);
        let stop =
            qty > Decimal::ZERO && price * qty <= cost * (Decimal::ONE - l.stop_loss_fraction);
        if qty > Decimal::ZERO && (decision.action == "SELL" || stop) {
            let execution = fill_price(price, false, l);
            let gross = qty * execution;
            let fee = gross * fee_rate;
            let net = gross - fee;
            pnls.push(f(net - cost));
            cash += net;
            fees += fee;
            slip += qty * (price - execution);
            qty = Decimal::ZERO;
            cost = Decimal::ZERO;
        } else if qty == Decimal::ZERO && decision.action == "BUY" {
            let execution = fill_price(price, true, l);
            qty = quantity(
                cash.min(l.order_budget),
                execution,
                d("0.00000001"),
                l.fee_bps,
            );
            let fee = qty * execution * fee_rate;
            cost = qty * execution + fee;
            cash -= cost;
            fees += fee;
            slip += qty * (execution - price);
        }
        if qty > Decimal::ZERO {
            exposed += 1;
        }
        let equity = f(cash + qty * dec(data[i].close));
        peak = peak.max(equity);
        dd = dd.max((peak - equity) / peak);
        curve.push(equity);
    }
    let gains: f64 = pnls.iter().filter(|x| **x > 0.0).sum();
    let losses: f64 = -pnls.iter().filter(|x| **x < 0.0).sum::<f64>();
    let realized: f64 = pnls.iter().sum();
    let returns: Vec<f64> = curve.windows(2).map(|x| x[1] / x[0] - 1.0).collect();
    let avg = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns.iter().map(|x| (x - avg).powi(2)).sum::<f64>() / returns.len() as f64;
    let first = dec(data[start].open);
    let last = dec(data[end - 1].close);
    let bq = quantity(
        l.initial_balance,
        fill_price(first, true, l),
        d("0.00000001"),
        l.fee_bps,
    );
    let bcost = bq * fill_price(first, true, l) * (Decimal::ONE + fee_rate);
    let bend =
        l.initial_balance - bcost + bq * fill_price(last, false, l) * (Decimal::ONE - fee_rate);
    // Reference: SMA10/SMA30, same next-open timing and costs; full available capital.
    let mut rcash = l.initial_balance;
    let mut rq = Decimal::ZERO;
    for i in start..end {
        let fast = data[i - 10..i].iter().map(|c| c.close).sum::<f64>() / 10.0;
        let slow = data[i - 30..i].iter().map(|c| c.close).sum::<f64>() / 30.0;
        let p = dec(data[i].open);
        if fast > slow && rq == Decimal::ZERO {
            let price = fill_price(p, true, l);
            rq = quantity(rcash, price, d("0.00000001"), l.fee_bps);
            rcash -= rq * price * (Decimal::ONE + fee_rate);
        } else if fast < slow && rq > Decimal::ZERO {
            rcash += rq * fill_price(p, false, l) * (Decimal::ONE - fee_rate);
            rq = Decimal::ZERO;
        }
    }
    let initial = f(l.initial_balance);
    let trades = pnls.len();
    Ok(Report {
        simulated: true,
        dataset_size: n,
        split: split.into(),
        trades,
        realized_pnl: realized,
        unrealized_pnl: f(qty * last - cost),
        return_pct: (curve.last().unwrap() / initial - 1.0) * 100.0,
        buy_hold_return_pct: (f(bend) / initial - 1.0) * 100.0,
        sma_reference_return_pct: (f(rcash + rq * last) / initial - 1.0) * 100.0,
        fees: f(fees),
        slippage_cost: f(slip),
        win_rate: if trades > 0 {
            pnls.iter().filter(|x| **x > 0.0).count() as f64 / trades as f64
        } else {
            0.0
        },
        profit_factor: if losses > 0.0 {
            Some(gains / losses)
        } else {
            None
        },
        expectancy: if trades > 0 {
            realized / trades as f64
        } else {
            0.0
        },
        max_drawdown: dd,
        exposure: exposed as f64 / (end - start) as f64,
        sharpe: if variance > 0.0 {
            Some(avg / variance.sqrt() * (365.0_f64 * 24.0 * 60.0).sqrt())
        } else {
            None
        },
        equity_curve: curve,
        rules_version: crate::strategies::RULES.into(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn future_changes_do_not_change_past_equity() {
        let mut data: Vec<Candle> = (0..500)
            .map(|i| Candle {
                time: i * 60000,
                open: 100.0 + i as f64 / 10.0,
                close: 100.05 + i as f64 / 10.0,
                high: 101.0 + i as f64 / 10.0,
                low: 99.0 + i as f64 / 10.0,
                volume: 100.0,
            })
            .collect();
        let a = run(&data, &RiskLimits::default(), "test").unwrap();
        for c in &mut data[450..] {
            c.open *= 2.0;
            c.close *= 2.0;
        }
        let b = run(&data, &RiskLimits::default(), "test").unwrap();
        assert_eq!(&a.equity_curve[..51], &b.equity_curve[..51]);
        assert!(a.fees > 0.0);
    }
}
