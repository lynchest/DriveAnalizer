# DriveAnalizer: Teknik Mimari ve Teknoloji Yığını Raporu

## 1. Yönetici Özeti ve Teknoloji Yığını (Stack)

Aşağıdaki tablo, performans, tip güvenliği (type safety) ve çapraz platform uyumluluğu gözetilerek seçilmiş "Best-in-Class" teknolojileri özetler.

| Katman | Teknoloji / Kütüphane | Seçim Nedeni |
| :--- | :--- | :--- |
| **Core Framework** | **Tauri (v2)** | Electron'a kıyasla çok daha düşük RAM/CPU tüketimi ve Rust backend gücü. |
| **Dil (Backend)** | **Rust** | Bellek güvenliği, GC (Garbage Collector) duraksamaları olmaması, C++ seviyesinde hız. |
| **Sistem İzleme** | **sysinfo** & **heim** | Çapraz platform (Windows/Linux) donanım ve I/O metriklerine erişim için standart. |
| **Veritabanı** | **SQLite** + **SQLx** | Gömülü, sunucusuz SQL motoru. SQLx, `async` desteği ve compile-time query checking sağlar. |
| **Frontend** | **React** + **Vite** + **TypeScript** | Hızlı derleme (HMR), geniş ekosistem ve bileşen bazlı mimari. |
| **Görselleştirme** | **uPlot** | Canvas tabanlı, binlerce veri noktasını milisaniyeler içinde çizebilen en hızlı grafik kütüphanesi. |
| **State Mgmt** | **Zustand** | Redux'a göre çok daha hafif, boilerplate içermeyen state yönetimi. |

---

## 2. Veri Toplama Katmanı (Backend - Rust)

Disk I/O izleme işlemi, ana UI thread'ini (Main Loop) asla bloklamamalıdır. Bu nedenle **Tokio** runtime üzerinde çalışan ayrı bir thread (worker) yapısı kuracağız.

*   **Kütüphane Seçimi: `sysinfo`**:
    *   Windows ve Linux üzerinde disk okuma/yazma hızlarını (bytes/sec) ve toplam transfer miktarlarını almak için en kararlı "crate"tir.
    *   *Alternatif:* Daha derinlemesine, process bazlı I/O takibi gerekirse Linux için `/proc` dosya sistemi parse edilmeli, Windows için ise `windows-rs` üzerinden **Performance Counters (PDH)** kullanılmalıdır. Ancak genel dashboard için `sysinfo` yeterli ve güvenlidir.

*   **Toplama Stratejisi (Polling Loop):**
    *   Veriler olay tabanlı (event-driven) değil, **örnekleme (sampling)** yöntemiyle toplanmalıdır.
    *   Önerilen örnekleme hızı: **1000ms (1 saniye)**. Daha sık örnekleme (örn. 100ms), sistem kaynaklarını gereksiz tüketir ve görselleştirmede "jitter" (titreme) yaratır.

## 3. Veri Yönetimi ve Persistence (SQLite & SQLx)

En kritik darboğaz burasıdır. Saniyede bir veri yazmak, eğer doğru yapılmazsa diski yorar ve izlediğimiz metrikleri bozar (Heisenberg İlkesi: Gözlemci, gözlemlenen sistemi etkiler).

*   **ORM/Query Builder: `SQLx`**:
    *   `Rusqlite` senkrondur ve I/O sırasında thread'i bloklar. `SQLx` ise tamamen **asenkron (async)** çalışır ve Rust'ın `Future` yapısıyla mükemmel uyum sağlar.
    *   Compile-time SQL doğrulama özelliği sayesinde runtime hatalarını minimize eder.

*   **Yazma Stratejisi: In-Memory Buffering & Batch Insert**:
    *   Her saniye veritabanına `INSERT` atmak **yasaktır**.
    *   **Çözüm:** Veriler Rust tarafında bir `Vec<DiskStat>` tamponunda (buffer) tutulur.
    *   Tampon dolduğunda (örn. her 60 saniyede bir veya 100 kayıtta bir) tek bir **Transaction** içinde toplu olarak (Batch Insert) SQLite'a yazılır.

*   **SQLite Optimizasyonu (PRAGMA Ayarları):**
    *   `JOURNAL_MODE = WAL` (Write-Ahead Logging): Okuma ve yazma işlemlerinin birbirini bloklamasını engeller. Eşzamanlılık için zorunludur.
    *   `SYNCHRONOUS = NORMAL`: Veri güvenliği ile performans arasındaki en iyi denge.

## 4. İletişim Köprüsü (IPC: Rust ↔ Frontend)

Tauri'de iki ana iletişim yöntemi vardır: `Invoke` (Command) ve `Emit` (Event). Gerçek zamanlı akış için **Event** modeli kullanılacaktır.

*   **Veri Akışı (Push Model):**
    *   Frontend sürekli "veri var mı?" diye sormaz (Polling yapmaz).
    *   Rust backend, veriyi topladığı anda `window.emit("io-update", payload)` fonksiyonu ile veriyi Frontend'e "iter" (push).

*   **Payload Optimizasyonu:**
    *   Veri yapısı `Serde` kütüphanesi ile serialize edilir.
    *   JSON yerine **Binary** veri göndermek mümkündür ancak bu ölçekte JSON'un overhead'i ihmal edilebilir düzeydedir ve geliştirme kolaylığı sağlar.

## 5. Arayüz Tasarımı (Frontend)

Görselleştirme katmanı, CPU'yu yormadan yüksek frekanslı güncellemeleri çizebilmelidir.

*   **Framework: React + Vite**:
    *   Sanal DOM (Virtual DOM) maliyetini düşürmek için `React.memo` ve `useMemo` hook'ları agresif şekilde kullanılmalıdır.

*   **Grafik Kütüphanesi: `uPlot`**:
    *   Neden Chart.js veya Recharts değil? Çünkü bunlar SVG veya ağır Canvas wrapper'ları kullanır. Binlerce veri noktası eklendiğinde tarayıcıyı dondururlar.
    *   **uPlot**, mikro-optimize edilmiş, WebGL gerektirmeyen ancak WebGL hızına yaklaşan, sadece Canvas kullanan bir kütüphanedir. Gerçek zamanlı osiloskop benzeri akışlar için endüstri standardıdır.

## 6. Mimari Sentez ve Veri Akış Diyagramı

Sistemin çalışma prensibi aşağıdaki "Producer-Consumer" (Üretici-Tüketici) modeline dayanır:

1.  **Collector Thread (Rust):** `sysinfo` ile her 1 saniyede bir CPU/Disk verilerini okur.
2.  **Data Cloning:** Okunan veri ikiye kopyalanır:
    *   *Kopya 1:* **IPC Kanalı** üzerinden anında Frontend'e fırlatılır (Canlı izleme için).
    *   *Kopya 2:* **Memory Buffer**'a eklenir (Kalıcı depolama için).
3.  **Persistence Thread (Rust):** Buffer doluluğunu kontrol eder. Eşik aşılırsa `SQLx` ile SQLite'a "Bulk Insert" yapar.
4.  **Frontend (React):** `Tauri Event Listener` gelen veriyi yakalar. React State'ini güncellemeden doğrudan `uPlot` instance'ının `setData` metodunu çağırır (React render döngüsünü bypass etmek performansı artırır).

### Özet Mimari Şeması

```mermaid
graph TD
    subgraph "Rust Backend (Tauri Core)"
        OS[İşletim Sistemi API] -->|sysinfo| Collector[Veri Toplayıcı (Loop)]
        Collector -->|Anlık Veri| IPC[Tauri Event Emitter]
        Collector -->|Veri Biriktirme| Buffer[Memory Buffer (Vec)]
        Buffer -->|Batch Doldu| DB_Writer[Async SQLx Writer]
        DB_Writer -->|Transaction| SQLite[(SQLite DB - WAL Mode)]
    end

    subgraph "Frontend (Webview)"
        IPC -->|JSON Payload| Listener[Event Listener]
        Listener -->|Direct Update| Chart[uPlot Canvas]
        Listener -->|State Update| Dashboard[React UI Paneli]
    end
```

## 7. Sonuç ve Tavsiyeler

Bu mimari, **DriveAnalizer** uygulamasının sistem kaynaklarını %1'in altında kullanarak çalışmasını garanti eder.

*   **Kritik Uyarı:** Windows'ta Antivirüs yazılımları, uygulamanın sürekli disk istatistiklerini sorgulamasını şüpheli davranış olarak algılayabilir. Uygulama imzalanmalı (Code Signing) ve gerekirse "Administrator" yetkileriyle çalıştırılacak şekilde manifest ayarlanmalıdır.
*   **Geliştirme Sırası:** Önce Rust tarafındaki veri toplama ve SQLite katmanını (Backend) CLI olarak yazıp test etmeni, stabilite sağlandıktan sonra Tauri arayüzünü (Frontend) entegre etmeni öneririm.
