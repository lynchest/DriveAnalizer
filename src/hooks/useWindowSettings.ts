import { useEffect } from 'react';
import { useStore } from '../store/useStore';

export function useWindowSettings() {
    const { windowSize } = useStore();

    useEffect(() => {
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
    }, [windowSize]);

    useEffect(() => {
        const handleContextMenu = (e: MouseEvent) => {
            e.preventDefault();
        };
        document.addEventListener('contextmenu', handleContextMenu);

        return () => {
            document.removeEventListener('contextmenu', handleContextMenu);
        };
    }, []);
}
