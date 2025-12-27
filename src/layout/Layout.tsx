import React, { ReactNode } from 'react';
import { useStore } from '../store/useStore';
import './Layout.css';

interface LayoutProps {
    children: ReactNode;
}

const Layout: React.FC<LayoutProps> = ({ children }) => {
    const { activePage, setActivePage, appMetrics } = useStore();

    return (
        <div className="layout">
            <aside className="layout__sidebar">
                <div className="layout__logo">
                    Drive<span>Analizer</span>
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

function formatBytes(bytes: number, decimals = 2) {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}

