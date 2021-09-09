pub mod map;

// calculate statistics from data, then return (min, avg, max, std)
pub fn calculate_stat(mut data: Vec<f64>) -> (f64, f64, f64, f64) {
    data.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let avg = data.iter().sum::<f64>() / (data.len() as f64);
    let var = data
        .iter()
        .map(|x| (x.max(avg) - x.min(avg)).powf(2.) / (data.len() as f64))
        .sum::<f64>();

    (data[0], avg, data[data.len() - 1], f64::sqrt(var))
}
