import React, { ReactNode, useState } from 'react';
import { useStore } from '../store/useStore';
import { formatBytes } from '../utils/format';
import './Layout.css';
import logo from '../assets/logo.png';

interface LayoutProps {
    children: ReactNode;
}

type VerificationStatus = 'idle' | 'valid' | 'invalid';

const Layout: React.FC<LayoutProps> = ({ children }) => {
    const { activePage, setActivePage, appMetrics, topProcesses, dataDisplayMode, processHistory, currentStats, allTimeTotals } = useStore();
    const [verificationStatus, setVerificationStatus] = useState<VerificationStatus>('idle');

    const handleVerifyData = () => {
        // Dashboard verilerini hesapla
        let dashboardRead = 0;
        let dashboardWrite = 0;

        if (dataDisplayMode === 'session') {
            dashboardRead = currentStats.read_bytes;
            dashboardWrite = currentStats.write_bytes;
        } else {
            dashboardRead = allTimeTotals.read_bytes + currentStats.read_bytes;
            dashboardWrite = allTimeTotals.write_bytes + currentStats.write_bytes;
        }

        // Top Processes toplamlarını hesapla
        let processReadTotal = 0;
        let processWriteTotal = 0;
        topProcesses.forEach(p => {
            processReadTotal += p.read_bytes;
            processWriteTotal += p.write_bytes;
        });

        // Eğer all-time modundaysak, geçmiş verileri de ekle
        if (dataDisplayMode === 'alltime') {
            Object.values(processHistory).forEach(stats => {
                processReadTotal += stats.read_bytes;
                processWriteTotal += stats.write_bytes;
            });
        }

        // Verileri karşılaştır
        const isReadEqual = dashboardRead === processReadTotal;
        const isWriteEqual = dashboardWrite === processWriteTotal;
        const isValid = isReadEqual && isWriteEqual;

        console.log('===== VERI DOĞRULAMA SONUCU =====');
        console.log(`📊 Dashboard Read: ${formatBytes(dashboardRead)}`);
        console.log(`📋 Process Read: ${formatBytes(processReadTotal)}`);
        console.log(`✓ Read Eşit: ${isReadEqual ? '✅ EVET' : '❌ HAYIR'}`);
        console.log('');
        console.log(`📊 Dashboard Write: ${formatBytes(dashboardWrite)}`);
        console.log(`📋 Process Write: ${formatBytes(processWriteTotal)}`);
        console.log(`✓ Write Eşit: ${isWriteEqual ? '✅ EVET' : '❌ HAYIR'}`);
        console.log('');
        console.log(`🎯 SONUÇ: ${isValid ? '✅ VERİLER EŞİT' : '❌ VERİLER EŞİT DEĞİL'}`);

        setVerificationStatus(isValid ? 'valid' : 'invalid');

        // 5 saniye sonra sıfırla
        setTimeout(() => setVerificationStatus('idle'), 5000);
    };

    return (
        <div className="layout">
            <aside className="layout__sidebar">
                <div className="layout__logo">
                    <img src={logo} alt="DriveAnalizer" className="layout__logo-img" />
                    <div className="layout__logo-text">Drive<span>Analizer</span></div>
                </div>
                <nav className="layout__nav">
                    <a 
                        href="#" 
                        className={`nav-item ${activePage === 'dashboard' ? 'active' : ''}`}
                        onClick={(e) => { e.preventDefault(); setActivePage('dashboard'); }}
                    >
                        Dashboard
                    </a>
                    <a 
                        href="#" 
                        className={`nav-item ${activePage === 'processes' ? 'active' : ''}`}
                        onClick={(e) => { e.preventDefault(); setActivePage('processes'); }}
                    >
                        Top Processes
                    </a>
                    <a 
                        href="#" 
                        className={`nav-item ${activePage === 'settings' ? 'active' : ''}`}
                        onClick={(e) => { e.preventDefault(); setActivePage('settings'); }}
                    >
                        Settings
                    </a>
                    {/* <button 
                        onClick={handleVerifyData}
                        className={`nav-item nav-item--check ${verificationStatus === 'valid' ? 'nav-item--valid' : verificationStatus === 'invalid' ? 'nav-item--invalid' : ''}`}
                        title="Verileri Doğrula"
                    >
                        {verificationStatus === 'valid' ? '✅' : verificationStatus === 'invalid' ? '❌' : '🔍'} Check Data
                    </button> */}
                </nav>

                <div className="layout__footer">
                    <div className="app-metrics">
                        <div className="metric-item">
                            <span className="metric-label">CPU:</span>
                            <span className="metric-value">{appMetrics.cpu_usage.toFixed(1)}%</span>
                        </div>
                        <div className="metric-item">
                            <span className="metric-label">RAM:</span>
                            <span className="metric-value">{formatBytes(appMetrics.ram_usage)}</span>
                        </div>
                        <div className="metric-item">
                            <span className="metric-label">Disk:</span>
                            <span className="metric-value">{formatBytes(appMetrics.total_disk_size)}</span>
                        </div>
                    </div>
                </div>
            </aside>
            <main className="layout__content">
                <header className="layout__header">
                    <h1>
                        {activePage === 'dashboard' && 'System I/O Monitor'}
                        {activePage === 'processes' && 'Top Processes'}
                        {activePage === 'settings' && 'Settings'}
                    </h1>
                </header>
                <div className="layout__body">
                    {children}
                </div>
            </main>
        </div>
    );
};

export default Layout;

