// Native Windows Performance Counter API wrapper
// PowerShell subprocess overhead'ini ortadan kaldırır
// Fallback: PowerShell veya varsayılan değerler

#[cfg(windows)]
mod windows_impl {
    use windows::core::PCWSTR;
    use windows::Win32::System::Performance::*;

    /// Disk performans metriklerini Windows PDH API ile al
    pub fn get_disk_perf_metrics() -> Result<(f64, f64), String> {
        unsafe {
            // Query handle oluştur
            let mut query_handle: isize = 0;
            let status = PdhOpenQueryW(PCWSTR::null(), 0, &mut query_handle);
            if status != 0 {
                return Err(format!("PdhOpenQueryW failed: {}", status));
            }

            // Counter path'leri
            let idle_path: Vec<u16> = "\\PhysicalDisk(_Total)\\% Idle Time\0"
                .encode_utf16()
                .collect();
            let queue_path: Vec<u16> = "\\PhysicalDisk(_Total)\\Avg. Disk Queue Length\0"
                .encode_utf16()
                .collect();

            // Counter'ları ekle (PdhAddEnglishCounterW kullanarak her dilde çalışmasını sağla)
            let mut idle_counter: isize = 0;
            let mut queue_counter: isize = 0;

            let status = PdhAddEnglishCounterW(
                query_handle,
                PCWSTR::from_raw(idle_path.as_ptr()),
                0,
                &mut idle_counter,
            );
            if status != 0 {
                PdhCloseQuery(query_handle);
                return Err(format!("PdhAddEnglishCounterW (idle) failed: {}", status));
            }

            let status = PdhAddEnglishCounterW(
                query_handle,
                PCWSTR::from_raw(queue_path.as_ptr()),
                0,
                &mut queue_counter,
            );
            if status != 0 {
                PdhCloseQuery(query_handle);
                return Err(format!("PdhAddEnglishCounterW (queue) failed: {}", status));
            }

            // İlk sorgu (baseline için)
            let status = PdhCollectQueryData(query_handle);
            if status != 0 {
                PdhCloseQuery(query_handle);
                return Err(format!("PdhCollectQueryData (1) failed: {}", status));
            }

            // Kısa bekleme (100ms) - performans counter'ları için gerekli
            std::thread::sleep(std::time::Duration::from_millis(100));

            // İkinci sorgu (gerçek değerler)
            let status = PdhCollectQueryData(query_handle);
            if status != 0 {
                PdhCloseQuery(query_handle);
                return Err(format!("PdhCollectQueryData (2) failed: {}", status));
            }

            // Değerleri al
            let mut idle_value = PDH_FMT_COUNTERVALUE::default();
            let mut queue_value = PDH_FMT_COUNTERVALUE::default();

            let status =
                PdhGetFormattedCounterValue(idle_counter, PDH_FMT_DOUBLE, None, &mut idle_value);
            let idle_time = if status == 0 {
                idle_value.Anonymous.doubleValue
            } else {
                0.0
            };

            let status =
                PdhGetFormattedCounterValue(queue_counter, PDH_FMT_DOUBLE, None, &mut queue_value);
            let queue_depth = if status == 0 {
                queue_value.Anonymous.doubleValue
            } else {
                0.0
            };

            // Temizlik
            PdhCloseQuery(query_handle);

            Ok((idle_time, queue_depth))
        }
    }
}

#[cfg(windows)]
pub use windows_impl::get_disk_perf_metrics;

/// Windows dışı platformlar için fallback
#[cfg(not(windows))]
pub fn get_disk_perf_metrics() -> Result<(f64, f64), String> {
    // Linux/macOS için henüz implemente edilmedi
    // Varsayılan değerler döndür
    Ok((100.0, 0.0))
}

/// Güvenli wrapper - hata durumunda varsayılan değerler
pub fn get_disk_perf_metrics_safe() -> (f64, f64) {
    match get_disk_perf_metrics() {
        Ok(metrics) => metrics,
        Err(e) => {
            eprintln!("[PerfCounters] Error: {}. Using defaults.", e);
            (100.0, 0.0) // Varsayılan: %100 idle, 0 queue
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_metrics_safe() {
        let (idle, queue) = get_disk_perf_metrics_safe();
        assert!(idle >= 0.0 && idle <= 100.0);
        assert!(queue >= 0.0);
    }
}
