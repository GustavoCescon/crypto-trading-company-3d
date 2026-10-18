use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub fn d(s: &str) -> Decimal {
    Decimal::from_str(s).expect("decimal constant")
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskLimits {
    pub initial_balance: Decimal,
    pub order_budget: Decimal,
    pub max_asset_fraction: Decimal,
    pub max_total_fraction: Decimal,
    pub daily_loss_fraction: Decimal,
    pub max_drawdown_fraction: Decimal,
    pub stop_loss_fraction: Decimal,
    pub take_profit_fraction: Decimal,
    pub max_positions: usize,
    pub max_signal_age_seconds: i64,
    pub fee_bps: Decimal,
    pub spread_bps: Decimal,
    pub slippage_bps: Decimal,
}
impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            initial_balance: d("10000"),
            order_budget: d("500"),
            max_asset_fraction: d("0.15"),
            max_total_fraction: d("0.6"),
            daily_loss_fraction: d("0.03"),
            max_drawdown_fraction: d("0.1"),
            stop_loss_fraction: d("0.025"),
            take_profit_fraction: d("0.04"),
            max_positions: 6,
            max_signal_age_seconds: 120,
            fee_bps: d("10"),
            spread_bps: d("2"),
            slippage_bps: d("3"),
        }
    }
}
impl RiskLimits {
    pub fn validate(&self) -> Result<(), String> {
        if self.initial_balance <= Decimal::ZERO
            || self.order_budget <= Decimal::ZERO
            || self.max_positions == 0
            || !(1..=600).contains(&self.max_signal_age_seconds)
        {
            return Err("invalid balance, budget, positions or signal age".into());
        }
        for v in [
            self.max_asset_fraction,
            self.max_total_fraction,
            self.daily_loss_fraction,
            self.max_drawdown_fraction,
            self.stop_loss_fraction,
            self.take_profit_fraction,
        ] {
            if v <= Decimal::ZERO || v > Decimal::ONE {
                return Err("risk fractions must be in (0,1]".into());
            }
        }
        for v in [self.fee_bps, self.spread_bps, self.slippage_bps] {
            if v < Decimal::ZERO || v > d("100") {
                return Err("cost basis points must be in [0,100]".into());
            }
        }
        Ok(())
    }
}
pub struct Exposure {
    pub cash: Decimal,
    pub reserved: Decimal,
    pub equity: Decimal,
    pub asset: Decimal,
    pub total: Decimal,
    pub positions: usize,
    pub daily_loss: Decimal,
    pub drawdown: Decimal,
}
pub fn veto(
    l: &RiskLimits,
    e: &Exposure,
    budget: Decimal,
    age: i64,
    quote_age: i64,
) -> Result<(), String> {
    if !(0..=l.max_signal_age_seconds).contains(&age) || !(0..=30).contains(&quote_age) {
        return Err("Sinal ou cotação fora da validade".into());
    }
    if e.equity <= Decimal::ZERO {
        return Err("Patrimônio inválido".into());
    }
    if e.daily_loss >= l.daily_loss_fraction {
        return Err("Limite de perda diária".into());
    }
    if e.drawdown >= l.max_drawdown_fraction {
        return Err("Limite de drawdown".into());
    }
    if e.positions >= l.max_positions && e.asset == Decimal::ZERO {
        return Err("Máximo de posições".into());
    }
    if budget > e.cash - e.reserved {
        return Err("Saldo disponível insuficiente".into());
    }
    if (e.asset + budget) / e.equity > l.max_asset_fraction {
        return Err("Exposição máxima por ativo".into());
    }
    if (e.total + e.reserved + budget) / e.equity > l.max_total_fraction {
        return Err("Exposição total máxima".into());
    }
    Ok(())
}
pub fn fill_price(mid: Decimal, buy: bool, l: &RiskLimits) -> Decimal {
    let adjustment = (l.spread_bps / d("2") + l.slippage_bps) / d("10000");
    mid * (Decimal::ONE + if buy { adjustment } else { -adjustment })
}
pub fn quantity(budget: Decimal, price: Decimal, step: Decimal, fee_bps: Decimal) -> Decimal {
    if price <= Decimal::ZERO || step <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    ((budget / (price * (Decimal::ONE + fee_bps / d("10000")))) / step).floor() * step
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn precision_and_costs() {
        let l = RiskLimits::default();
        let p = fill_price(d("100"), true, &l);
        assert_eq!(p, d("100.04"));
        let q = quantity(d("500"), p, d("0.001"), l.fee_bps);
        assert!(q * p * (Decimal::ONE + l.fee_bps / d("10000")) <= d("500"));
    }
    #[test]
    fn risk_reservations_and_stale_data() {
        let l = RiskLimits::default();
        let mut e = Exposure {
            cash: d("1000"),
            reserved: d("600"),
            equity: d("10000"),
            asset: d("0"),
            total: d("0"),
            positions: 0,
            daily_loss: d("0"),
            drawdown: d("0"),
        };
        assert!(veto(&l, &e, d("500"), 0, 0).is_err());
        e.reserved = d("0");
        assert!(veto(&l, &e, d("500"), 0, 0).is_ok());
        assert!(veto(&l, &e, d("500"), 0, 31).is_err());
        e.drawdown = d("0.2");
        assert!(veto(&l, &e, d("500"), 0, 0).is_err());
    }
}
