import { useMemo } from 'react';
import Layout from './layout/Layout';
import MetricCard from './components/MetricCard';
import Chart from './components/Chart';
import TopProcesses from './pages/TopProcesses';
import Settings from './pages/Settings';
import { useStore } from './store/useStore';
import { useAppMetrics } from './hooks/useAppMetrics';
import { useDiskMetrics } from './hooks/useDiskMetrics';
import { useWindowSettings } from './hooks/useWindowSettings';
import { useDataSync } from './hooks/useDataSync';
import { formatBytes } from './utils/format';
import './App.css';

function App() {
    const { 
        currentStats, 
        history, 
        activePage, 
        dataDisplayMode,
        allTimeTotals
    } = useStore();

    // Initialize hooks
    useAppMetrics();
    useDiskMetrics();
    useWindowSettings();
    useDataSync();

    const { displayReadBytes, displayWriteBytes } = useMemo(() => {
        if (dataDisplayMode === 'session') {
            return {
                displayReadBytes: currentStats.read_bytes,
                displayWriteBytes: currentStats.write_bytes
            };
        }
        
        return {
            displayReadBytes: allTimeTotals.read_bytes + currentStats.read_bytes,
            displayWriteBytes: allTimeTotals.write_bytes + currentStats.write_bytes
        };
    }, [dataDisplayMode, currentStats.read_bytes, currentStats.write_bytes, allTimeTotals]);

    const renderPage = () => {
        switch (activePage) {
            case 'dashboard':
                return (
                    <>
                        <div className="dashboard-grid">
                            <MetricCard 
                                label="Read Speed" 
                                value={formatBytes(currentStats.read_speed) + '/s'} 
                                unit="Current" 
                                variant="primary" 
                            />
                            <MetricCard 
                                label="Write Speed" 
                                value={formatBytes(currentStats.write_speed) + '/s'} 
                                unit="Current" 
                                variant="secondary" 
                            />
                            <MetricCard 
                                label="Idle Time" 
                                value={currentStats.idle_time.toFixed(1)} 
                                unit="%" 
                                variant={
                                    currentStats.idle_time > 80 ? 'success' : 
                                    currentStats.idle_time > 30 ? 'warning' : 'danger'
                                } 
                            />
                            <MetricCard 
                                label="Total Read" 
                                value={formatBytes(displayReadBytes)} 
                                unit={dataDisplayMode === 'session' ? 'Session' : 'Total'} 
                                variant="primary" 
                            />
                            <MetricCard 
                                label="Total Write" 
                                value={formatBytes(displayWriteBytes)} 
                                unit={dataDisplayMode === 'session' ? 'Session' : 'Total'} 
                                variant="secondary" 
                            />
                            <MetricCard 
                                label="Queue Depth" 
                                value={currentStats.queue_depth.toFixed(2)} 
                                unit="Count" 
                                variant={
                                    currentStats.queue_depth < 1.0 ? 'success' : 
                                    currentStats.queue_depth < 4.0 ? 'warning' : 'danger'
                                } 
                            />
                        </div>
                        <Chart data={history} />
                    </>
                );
            case 'processes':
                return <TopProcesses />;
            case 'settings':
                return <Settings />;
            default:
                return null;
        }
    };

    return (
        <Layout>
            {renderPage()}
        </Layout>
    );
}

export default App;


