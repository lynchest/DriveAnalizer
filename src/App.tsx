import { useEffect, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import Layout from './layout/Layout';
import MetricCard from './components/MetricCard';
import Chart from './components/Chart';
import TopProcesses from './pages/TopProcesses';
import Settings from './pages/Settings';
import { useStore, DiskStat, AllTimeTotals, AppMetrics } from './store/useStore';
import './App.css';

function App() {
  const { 
    currentStats, 
    history, 
    activePage, 
    updateStats, 
    windowSize, 
    dataDisplayMode,
    allTimeTotals,
    setAllTimeTotals,
    setAppMetrics
  } = useStore();

  // Fetch all-time totals from backend
  const fetchAllTimeTotals = useCallback(async () => {
    try {
      const totals = await invoke<AllTimeTotals>('get_alltime_totals');
      setAllTimeTotals(totals);
    } catch (error) {
      console.error('Failed to fetch all-time totals:', error);
    }
  }, [setAllTimeTotals]);

  const fetchAppMetrics = useCallback(async () => {
    try {
      const metrics = await invoke<AppMetrics>('get_app_metrics');
      setAppMetrics(metrics);
    } catch (error) {
      console.error('Failed to fetch app metrics:', error);
    }
  }, [setAppMetrics]);

  useEffect(() => {
    fetchAppMetrics();
    const interval = setInterval(fetchAppMetrics, 2000);
    return () => clearInterval(interval);
  }, [fetchAppMetrics]);

  useEffect(() => {
    // Apply persisted window size on startup
    const applySettings = async () => {
      try {
        const { LogicalSize, getCurrentWindow } = await import('@tauri-apps/api/window');
        const appWindow = getCurrentWindow();
        await appWindow.setSize(new LogicalSize(windowSize.width, windowSize.height));
      } catch (error) {
        console.error('Failed to apply window settings:', error);
      }
    };
    applySettings();
  }, [windowSize]); // Run when size changes

  useEffect(() => {
    // Disable right-click context menu
    const handleContextMenu = (e: MouseEvent) => {
      e.preventDefault();
    };
    document.addEventListener('contextmenu', handleContextMenu);

    return () => {
      document.removeEventListener('contextmenu', handleContextMenu);
    };
  }, []);

  useEffect(() => {
    // Listen to backend events
    const unlisten = listen<DiskStat>('disk-metrics', (event) => {
      updateStats(event.payload);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [updateStats]);

  // Fetch all-time totals when mode changes to 'alltime' or on mount
  useEffect(() => {
    fetchAllTimeTotals();
  }, [fetchAllTimeTotals]);

  // Determine which values to show based on mode
  const displayReadBytes = dataDisplayMode === 'session' 
    ? currentStats.read_bytes 
    : allTimeTotals.read_bytes + currentStats.read_bytes;
  
  const displayWriteBytes = dataDisplayMode === 'session' 
    ? currentStats.write_bytes 
    : allTimeTotals.write_bytes + currentStats.write_bytes;

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
                unit={dataDisplayMode === 'session' ? 'Oturum' : 'Toplam'} 
                variant="primary" 
              />
              <MetricCard 
                label="Total Write" 
                value={formatBytes(displayWriteBytes)} 
                unit={dataDisplayMode === 'session' ? 'Oturum' : 'Toplam'} 
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
            
            {/* Debug/Info Section */}
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

// Reuse helper (could be moved to utils)
function formatBytes(bytes: number, decimals = 2) {
  if (!+bytes) return '0 B';
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
}

export default App;


