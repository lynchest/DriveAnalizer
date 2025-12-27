import React, { useState, useEffect } from 'react';
// Removed unused imports to prevent potential load issues if they fail
import { invoke } from '@tauri-apps/api/core';
import { useStore, DataDisplayMode, AllTimeTotals } from '../store/useStore';
import './Settings.css';

const Settings: React.FC = () => {
    const { 
        windowSize, setWindowSize, 
        dataDisplayMode, setDataDisplayMode,
        resetSessionData,
        setAllTimeTotals,
        currentStats
    } = useStore();
    const [resetting, setResetting] = useState(false);
    const [dbSize, setDbSize] = useState<number | null>(null);
    const [loadingDbSize, setLoadingDbSize] = useState(true);
    const [customWidth, setCustomWidth] = useState(windowSize.width.toString());
    const [customHeight, setCustomHeight] = useState(windowSize.height.toString());
    
    // Load database size on mount
    useEffect(() => {
        loadDatabaseSize();
    }, []);

    useEffect(() => {
        setCustomWidth(windowSize.width.toString());
        setCustomHeight(windowSize.height.toString());
    }, [windowSize]);

    const loadDatabaseSize = async () => {
        try {
            setLoadingDbSize(true);
            const response = await invoke<{ db_size_before: number; db_size_after: number }>('get_database_size');
            setDbSize(response.db_size_before);
        } catch (error) {
            console.error('Failed to load database size:', error);
            setDbSize(null);
        } finally {
            setLoadingDbSize(false);
        }
    };
    
    const handleResize = async (width: number, height: number) => {
        try {
            // Dynamic import to match App.tsx pattern and avoid load-time errors
            const { LogicalSize, getCurrentWindow } = await import('@tauri-apps/api/window');
            const appWindow = getCurrentWindow();
            await appWindow.setSize(new LogicalSize(width, height));
            setWindowSize(width, height);
        } catch (error) {
            console.error('Failed to resize window:', error);
        }
    };

    const handleResetDatabase = async () => {
        if (!window.confirm('Veritabanındaki tüm veriler silinecek. Emin misiniz?')) return;
        
        setResetting(true);
        try {
            const response = await invoke<{ db_size_before: number; db_size_after: number }>('reset_database');
            
            resetSessionData(); // Mevcut oturumdaki verileri de sıfırla
            setAllTimeTotals({ read_bytes: 0, write_bytes: 0 });
            
            // Update database size display
            setDbSize(response.db_size_after);
            
            alert('Veritabanı ve oturum verileri başarıyla sıfırlandı.');
        } catch (error) {
            console.error('Reset error:', error);
            alert('Sıfırlama sırasında bir hata oluştu.');
        } finally {
            setResetting(false);
        }
    };

    const isSelected = (w: number, h: number) => 
        windowSize.width === w && windowSize.height === h;

    const isModeSelected = (mode: DataDisplayMode) => dataDisplayMode === mode;

    return (
        <div className="settings-page">
            {/* Data Display Mode Section */}
            <section className="settings-section">
                <h2>Veri Görüntüleme Modu</h2>
                <p className="settings-description">
                    Dashboard ve Top Processes sayfalarında gösterilecek verilerin kapsamını seçin.
                </p>
                
                <div className="mode-options">
                    <button 
                        className={`mode-btn ${isModeSelected('session') ? 'active' : ''}`}
                        onClick={() => setDataDisplayMode('session')}
                    >
                        <div className="mode-icon">🕐</div>
                        <div className="mode-content">
                            <div className="mode-label">Oturum (Session)</div>
                            <div className="mode-description">
                                Yalnızca bu oturumda (uygulama açıldığından beri) toplanan veriler gösterilir.
                            </div>
                        </div>
                    </button>
                    
                    <button 
                        className={`mode-btn ${isModeSelected('alltime') ? 'active' : ''}`}
                        onClick={() => setDataDisplayMode('alltime')}
                    >
                        <div className="mode-icon">📊</div>
                        <div className="mode-content">
                            <div className="mode-label">Tüm Zamanlar</div>
                            <div className="mode-description">
                                Veritabanında kayıtlı tüm geçmiş veriler toplanarak gösterilir.
                            </div>
                        </div>
                    </button>
                </div>
            </section>

            {/* Window and UI Section */}
            <section className="settings-section">
                <h2>Pencere Boyutu</h2>
                <p className="settings-description">Uygulama penceresinin boyutunu buradan ayarlayabilirsiniz.</p>
                
                <div className="size-options">
                    <button 
                        className={`size-btn ${isSelected(800, 600) ? 'active' : ''}`}
                        onClick={() => handleResize(800, 600)}
                    >
                        <div className="size-label">Küçük</div>
                        <div className="size-value">800 x 600</div>
                    </button>
                    
                    <button 
                        className={`size-btn ${isSelected(1024, 768) ? 'active' : ''}`}
                        onClick={() => handleResize(1024, 768)}
                    >
                        <div className="size-label">Orta</div>
                        <div className="size-value">1024 x 768</div>
                    </button>
                    
                    <button 
                        className={`size-btn ${isSelected(1280, 800) ? 'active' : ''}`}
                        onClick={() => handleResize(1280, 800)}
                    >
                        <div className="size-label">Büyük</div>
                        <div className="size-value">1280 x 800</div>
                    </button>

                    <button 
                        className={`size-btn ${isSelected(1366, 768) ? 'active' : ''}`}
                        onClick={() => handleResize(1366, 768)}
                    >
                        <div className="size-label">HD</div>
                        <div className="size-value">1366 x 768</div>
                    </button>

                    <button 
                        className={`size-btn ${isSelected(1920, 1080) ? 'active' : ''}`}
                        onClick={() => handleResize(1920, 1080)}
                    >
                        <div className="size-label">Full HD</div>
                        <div className="size-value">1920 x 1080</div>
                    </button>
                </div>

                <div className="custom-size-section">
                    <h3>Özel Boyut</h3>
                    <div className="custom-size-inputs">
                        <div className="input-group">
                            <label>Genişlik (px)</label>
                            <input 
                                type="number" 
                                value={customWidth} 
                                onChange={(e) => setCustomWidth(e.target.value)}
                                placeholder="Genişlik"
                            />
                        </div>
                        <div className="input-group">
                            <label>Yükseklik (px)</label>
                            <input 
                                type="number" 
                                value={customHeight} 
                                onChange={(e) => setCustomHeight(e.target.value)}
                                placeholder="Yükseklik"
                            />
                        </div>
                        <button 
                            className="apply-btn"
                            onClick={() => handleResize(parseInt(customWidth), parseInt(customHeight))}
                        >
                            Uygula
                        </button>
                    </div>
                </div>
            </section>

            {/* Maintenance Section */}
            <section className="settings-section">
                <h2>Bakım</h2>
                <p className="settings-description">Veritabanı yönetimi ve sistem temizliği.</p>
                
                <div className="maintenance-card">
                    <div className="card-header">
                        <div className="card-title">💾 Veritabanı Durumu</div>
                    </div>
                    <div className="card-content">
                        {loadingDbSize ? (
                            <p className="db-size-loading">Boyut hesaplanıyor...</p>
                        ) : dbSize !== null ? (
                            <div className="db-size-info">
                                <p className="db-size-label">Veritabanı Boyutu:</p>
                                <p className="db-size-value">
                                    {(dbSize / 1024 / 1024).toFixed(2)} MB
                                </p>
                            </div>
                        ) : (
                            <p className="db-size-error">Boyut bilgisi alınamadı</p>
                        )}
                    </div>
                </div>
                
                <div className="maintenance-actions">
                    <button 
                        className="danger-btn" 
                        onClick={handleResetDatabase}
                        disabled={resetting}
                    >
                        {resetting ? 'Sıfırlanıyor...' : 'Veritabanını Sıfırla'}
                    </button>
                </div>
            </section>

            <section className="settings-section">
                <h2>Hakkında</h2>
                <div className="about-card">
                    <p><strong>DriveAnalizer v0.1.0</strong></p>
                    <p>Yüksek performanslı disk izleme aracı.</p>
                </div>
            </section>
        </div>
    );
};

export default Settings;


