import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useStore, AllTimeTotals, ProcessInfo } from '../store/useStore';

export function useDataSync() {
    const { 
        setTopProcesses, 
        setAllTimeTotals, 
        setProcessHistory
    } = useStore();

    // Top processes event listener
    useEffect(() => {
        const unlistenPromise = listen<ProcessInfo[]>('top-processes', (event) => {
            setTopProcesses(event.payload);
        });

        return () => {
            unlistenPromise.then((unlisten) => unlisten());
        };
    }, [setTopProcesses]);

    // Database reset event listener
    useEffect(() => {
        const unlistenPromise = listen('database-reset', async () => {
            try {
                const totals = await invoke<AllTimeTotals>('get_alltime_totals');
                setAllTimeTotals(totals);
            } catch (error) {
                console.error('Failed to fetch totals after reset:', error);
            }

            try {
                const history = await invoke<Record<string, [number, number]>>('get_process_history');
                const formattedHistory: Record<string, { read_bytes: number, write_bytes: number }> = {};
                Object.entries(history).forEach(([name, [read, write]]) => {
                    formattedHistory[name] = { read_bytes: read, write_bytes: write };
                });
                setProcessHistory(formattedHistory);
            } catch (error) {
                console.error('Failed to fetch process history after reset:', error);
            }
        });

        return () => {
            unlistenPromise.then((unlisten) => unlisten());
        };
    }, [setAllTimeTotals, setProcessHistory]);

    // Initial fetch of all-time totals and process history
    useEffect(() => {
        const fetchData = async () => {
            try {
                const totals = await invoke<AllTimeTotals>('get_alltime_totals');
                setAllTimeTotals(totals);
            } catch (error) {
                console.error('Failed to fetch initial totals:', error);
            }

            try {
                const history = await invoke<Record<string, [number, number]>>('get_process_history');
                const formattedHistory: Record<string, { read_bytes: number, write_bytes: number }> = {};
                Object.entries(history).forEach(([name, [read, write]]) => {
                    formattedHistory[name] = { read_bytes: read, write_bytes: write };
                });
                setProcessHistory(formattedHistory);
            } catch (error) {
                console.error('Failed to fetch initial process history:', error);
            }
        };
        fetchData();
    }, [setAllTimeTotals, setProcessHistory]);
}
