use std::fs::File;
use std::io::{self, BufRead, BufReader};

// CALCULATE ANNUALIZED VOLATILITY FROM AN ASSET PRICE SERIES
pub fn calculate_historical_volatility(prices: &[f64]) -> Option<f64> {
    if prices.len() < 3 {
        return None;
    }

    // 1. Convert absolute prices to logarithmic returns: ln(P_t / P_{t-1})
    let mut log_returns = Vec::with_capacity(prices.len() - 1);
    for i in 1..prices.len() {
        if prices[i] <= 0.0 || prices[i - 1] <= 0.0 {
            return None; // Guard against negative or zero pricing data
        }
        log_returns.push((prices[i] / prices[i - 1]).ln());
    }

    // 2. Compute Mean Average of Logarithmic Returns
    let count = log_returns.len() as f64;
    let mean: f64 = log_returns.iter().sum::<f64>() / count;

    // 3. Compute Sample Variance
    let variance: f64 = log_returns
        .iter()
        .map(|&r| {
            let diff = r - mean;
            diff * diff
        })
        .sum::<f64>()
        / (count - 1.0);

    let daily_vol = variance.sqrt();

    // 4. Annualize assuming 252 typical market trading sessions per calendar year
    Some(daily_vol * (252.0_f64).sqrt())
}

// MOCK LOCAL STORAGE GENERATION UTILITY FOR RUNNING TESTING PIPELINES
pub fn generate_mock_price_file(file_path: &str) -> io::Result<()> {
    use std::io::Write;
    let mut file = File::create(file_path)?;
    // Simulating a minor moving upward trend starting at $100
    let mock_prices = vec![
        100.0, 101.5, 100.8, 102.1, 103.4, 102.9, 104.2, 103.8, 105.1, 106.3,
    ];
    for price in mock_prices {
        writeln!(file, "{}", price)?;
    }
    Ok(())
}

// PARSE LINE BY LINE DATA STRINGS INTO FLOATING ARRAYS SAFELY
pub fn parse_price_file(file_path: &str) -> io::Result<Vec<f64>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut prices = Vec::new();

    for line in reader.lines() {
        let line_str = line?;
        if let Ok(price) = line_str.trim().parse::<f64>() {
            prices.push(price);
        }
    }
    Ok(prices)
}
