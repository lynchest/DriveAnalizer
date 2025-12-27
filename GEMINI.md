# GEMINI.md - Proje Asistanı Rehberi

Bu dosya, **DriveAnalizer** projesi üzerinde çalışan yapay zeka asistanları (özellikle Gemini) için bağlam, kurallar ve öncelikleri belirler.

## 1. Temel İletişim Kuralı: Türkçe 🇹🇷

*   **Dil:** Kullanıcı ile etkileşimde **SADECE TÜRKÇE** kullanılacaktır.
*   **Ton:** Profesyonel, teknik açıdan yetkin ancak anlaşılır ve yardımsever.
*   **Kod Yorumları:** Kod içi yorumlar ve dokümantasyon İngilizce olabilir (standart gereği), ancak açıklama metinleri Türkçe olmalıdır.

## 2. Proje Kimliği ve Felsefesi

**DriveAnalizer**, sıradan bir disk izleme aracı değildir. Temel felsefesi **"Yüksek Performans ve Düşük Kaynak Tüketimi"**dir.

*   **Amaç:** Kullanıcının disk I/O performansını, sistemi yormadan (Heisenberg İlkesi'ne takılmadan) analiz etmek.
*   **Hedef:** Electron tabanlı hantal uygulamaların aksine, Rust'ın gücünü kullanarak minimum RAM ve CPU ile çalışmak.

## 3. Teknoloji Yığını ve Mimari Kararlar

Bu projede yapılan her teknik seçim, performans gerekçelerine dayanmaktadır. Asistan, kod önerirken bu mimariye sadık kalmalıdır.

| Alan | Teknoloji | Kritik Notlar |
| :--- | :--- | :--- |
| **Backend** | **Rust + Tauri v2** | Güvenlik ve hız. `tokio` ile asenkron yapı zorunludur. |
| **Veri Toplama** | **sysinfo** | Polling (örnekleme) yöntemiyle çalışır. Event-driven değildir. |
| **Veritabanı** | **SQLite + SQLx** | `WAL` modu açık olmalı. **Batch Insert** (Toplu Yazma) zorunludur. Tek tek insert yasaktır. |
| **Frontend** | **React + Vite + TS** | Hız için. Gereksiz re-render'lardan kaçınılmalı. |
| **Grafik** | **uPlot** | `Chart.js` veya `Recharts` KULLANILMAMALIDIR. Binlerce veri noktası için `uPlot` seçilmiştir. |
| **State** | **Zustand** | Redux kullanılmayacak. |

## 4. Geliştirme Kuralları ve Standartlar

### A. Performans Odaklı Kodlama
*   **Rust Tarafı:**
    *   Ana thread (Main Thread) asla bloklanmamalıdır. Ağır işler `tokio::spawn` ile ayrı task'lara taşınmalıdır.
    *   Veri tabanına yazarken "Buffer" mekanizması kullanılmalıdır (Örn: Verileri RAM'de biriktir, 60 saniyede bir yaz).
    *   `unwrap()` kullanımından kaçınılmalı, düzgün hata yönetimi (`Result`, `Option`) yapılmalıdır.

*   **Frontend Tarafı:**
    *   Grafik çizimlerinde canvas performansı gözetilmelidir.
    *   React bileşenlerinde `useMemo` ve `useCallback` gereksiz render'ları önlemek için aktif kullanılmalıdır.

### B. Dosya ve Klasör Yapısı
*   `src-tauri/src/monitor.rs`: Sistem izleme mantığı burada olmalı.
*   `src-tauri/src/models.rs`: Veri yapıları (Structs) burada tanımlanmalı.
*   `src-tauri/src/lib.rs`: Modül tanımları ve Tauri komutları burada toplanmalı.

## 5. Çalışma Yöntemi

1.  **Önce Analiz:** Kullanıcı bir özellik istediğinde, önce `DriveAnalizer_Architecture.md` ve `ROADMAP.md` dosyalarını kontrol et. Mimariye uygun mu?
2.  **Adım Adım İlerleme:** Karmaşık görevleri parçalara böl. Önce Backend (Rust), sonra Frontend (React) tarafını hallet.
3.  **Kullanıcıyı Bilgilendir:** Yaptığın işlemin performans etkisini kullanıcıya açıkla. (Örn: *"Bu veriyi her saniye diske yazmak yerine bellekte tutup toplu yazacağız, böylece diski yormayacağız."*)

## 6. Kritik Hatırlatmalar

*   ⚠️ **Asla** `Chart.js` önerme. Biz `uPlot` kullanıyoruz.
*   ⚠️ **Asla** senkron veritabanı sürücüsü (`rusqlite` vb.) önerme. Biz `sqlx` (async) kullanıyoruz.
*   ⚠️ **Asla** İngilizce cevap verme.

Bu dosya, projenin "Anayasası" niteliğindedir.
