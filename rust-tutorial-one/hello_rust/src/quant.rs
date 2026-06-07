use rand::random_range;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use statrs::distribution::{ContinuousCDF, Normal};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OptionInputs {
    pub stock_price: f64,
    pub strike_price: f64,
    pub time_to_maturity: f64,
    pub risk_free_rate: f64,
    pub volatility: f64,
}

#[derive(Deserialize, Debug)]
pub struct IvPayload {
    pub inputs: OptionInputs,
    pub observed_market_price: f64,
    pub is_call: bool,
}

#[derive(Debug, PartialEq)]
pub enum QuantError {
    InvalidStockPrice,
    InvalidStrikePrice,
    InvalidMaturity,
    InvalidVolatility,
    MathComputationError,
    IvConvergenceFailed,
}

impl fmt::Display for QuantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStockPrice => write!(f, "Stock price must be > 0.0"),
            Self::InvalidStrikePrice => write!(f, "Strike price must be > 0.0"),
            Self::InvalidMaturity => write!(f, "Time to maturity must be > 0.0"),
            Self::InvalidVolatility => write!(f, "Volatility must be > 0.0"),
            Self::MathComputationError => write!(f, "Mathematical domain or overflow error"),
            Self::IvConvergenceFailed => write!(
                f,
                "Newton-Raphson failed to converge on an Implied Volatility"
            ),
        }
    }
}

#[derive(Serialize)]
pub struct OptionGreeks {
    pub call_delta: f64,
    pub put_delta: f64,
    pub gamma: f64,
    pub vega: f64,
    pub call_theta: f64,
    pub put_theta: f64,
}

#[derive(Serialize)]
pub struct FullMetricsResponse {
    pub analytical_call: f64,
    pub analytical_put: f64,
    pub monte_carlo_call: f64,
    pub monte_carlo_put: f64,
    pub greeks: OptionGreeks,
    pub processing_time_ms: f64,
}

impl OptionInputs {
    pub fn validate(&self) -> Result<(), QuantError> {
        if self.stock_price <= 0.0 {
            return Err(QuantError::InvalidStockPrice);
        }
        if self.strike_price <= 0.0 {
            return Err(QuantError::InvalidStrikePrice);
        }
        if self.time_to_maturity <= 0.0 {
            return Err(QuantError::InvalidMaturity);
        }
        if self.volatility <= 0.0 {
            return Err(QuantError::InvalidVolatility);
        }
        Ok(())
    }

    pub fn calculate_greeks(&self) -> Result<(f64, f64, OptionGreeks), QuantError> {
        self.validate()?;
        let s = self.stock_price;
        let k = self.strike_price;
        let t = self.time_to_maturity;
        let r = self.risk_free_rate;
        let v = self.volatility;

        let d1 = ((s / k).ln() + (r + (v * v) / 2.0) * t) / (v * t.sqrt());
        let d2 = d1 - v * t.sqrt();

        let n = Normal::new(0.0, 1.0).map_err(|_| QuantError::MathComputationError)?;

        let n_d1 = n.cdf(d1);
        let n_d2 = n.cdf(d2);
        let pdf_d1 = (-0.5 * d1 * d1).exp() / (2.0 * std::f64::consts::PI).sqrt();

        let call_price = s * n_d1 - k * (-r * t).exp() * n_d2;
        let put_price = k * (-r * t).exp() * n.cdf(-d2) - s * n.cdf(-d1);

        let greeks = OptionGreeks {
            call_delta: n_d1,
            put_delta: n_d1 - 1.0,
            gamma: pdf_d1 / (s * v * t.sqrt()),
            vega: s * t.sqrt() * pdf_d1,
            call_theta: (-(s * pdf_d1 * v) / (2.0 * t.sqrt())) - r * k * (-r * t).exp() * n_d2,
            put_theta: (-(s * pdf_d1 * v) / (2.0 * t.sqrt())) + r * k * (-r * t).exp() * n.cdf(-d2),
        };

        Ok((call_price, put_price, greeks))
    }

    pub fn monte_carlo_price(&self, num_paths: usize) -> Result<(f64, f64), QuantError> {
        self.validate()?;
        let s = self.stock_price;
        let k = self.strike_price;
        let t = self.time_to_maturity;
        let r = self.risk_free_rate;
        let v = self.volatility;

        let drift = (r - (v * v) / 2.0) * t;
        let diffusion = v * t.sqrt();
        let n = Normal::new(0.0, 1.0).map_err(|_| QuantError::MathComputationError)?;

        let total_payoffs: (f64, f64) = (0..num_paths)
            .into_par_iter()
            .map(|_| {
                let u: f64 = random_range(0.0001..0.9999);
                let z = n.inverse_cdf(u);
                let st = s * (drift + diffusion * z).exp();
                let call_payoff = if st > k { st - k } else { 0.0 };
                let put_payoff = if k > st { k - st } else { 0.0 };
                (call_payoff, put_payoff)
            })
            .reduce(|| (0.0, 0.0), |a, b| (a.0 + b.0, a.1 + b.1));

        let discount_factor = (-r * t).exp();
        Ok((
            (total_payoffs.0 / num_paths as f64) * discount_factor,
            (total_payoffs.1 / num_paths as f64) * discount_factor,
        ))
    }

    // 5. NEWTON-RAPHSON IMPLIED VOLATILITY ENGINE
    // Iteratively checks price differences against Vega to discover the market-implied volatility
    pub fn calculate_implied_volatility(
        &self,
        market_price: f64,
        is_call: bool,
    ) -> Result<f64, QuantError> {
        let max_iterations = 100;
        let epsilon = 1e-6; // Target absolute precision
        let mut current_iv = 0.50; // Initial educated guess (50% volatility)

        for _ in 0..max_iterations {
            let mut test_inputs = self.clone();
            test_inputs.volatility = current_iv;

            let (call_p, put_p, greeks) = test_inputs.calculate_greeks()?;
            let calculated_price = if is_call { call_p } else { put_p };

            let price_difference = calculated_price - market_price;

            // Stop iterating if the difference is smaller than our precision target
            if price_difference.abs() < epsilon {
                return Ok(current_iv);
            }

            // Prevent dividing by zero if Vega drops to absolute zero
            if greeks.vega.abs() < 1e-4 {
                break;
            }

            // Newton-Raphson update rule: x_{n+1} = x_n - f(x_n) / f'(x_n)
            // In finance, the derivative of price with respect to volatility is Vega
            current_iv -= price_difference / greeks.vega;

            // Constrain IV to logical boundary bounds (0.01% to 500%)
            if current_iv <= 0.0001 || current_iv > 5.0 {
                break;
            }
        }

        Err(QuantError::IvConvergenceFailed)
    }
}
