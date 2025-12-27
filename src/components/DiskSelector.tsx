import React, { useState, useRef, useEffect } from 'react';
import { DiskStat } from '../store/useStore';
import './DiskSelector.css';

interface DiskSelectorProps {
    selectedDisk: string;
    availableDisks: string[];
    diskStats: Record<string, DiskStat>;
    onSelect: (disk: string) => void;
}

const DiskSelector: React.FC<DiskSelectorProps> = ({ selectedDisk, availableDisks, diskStats, onSelect }) => {
    const [isOpen, setIsOpen] = useState(false);
    const dropdownRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        const handleClickOutside = (event: MouseEvent) => {
            if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
                setIsOpen(false);
            }
        };

        document.addEventListener('mousedown', handleClickOutside);
        return () => {
            document.removeEventListener('mousedown', handleClickOutside);
        };
    }, []);

    const handleSelect = (disk: string) => {
        onSelect(disk);
        setIsOpen(false);
    };

    const formatSpeed = (bytes: number) => {
        if (!bytes) return '';
        const k = 1024;
        const result = bytes / k / k; // to MB
        if (result < 0.1) return '';
        return `${result.toFixed(1)} MB/s`;
    };

    return (
        <div className="disk-selector-container" ref={dropdownRef}>
            <button 
                className={`disk-selector-button ${isOpen ? 'active' : ''}`} 
                onClick={() => setIsOpen(!isOpen)}
                type="button"
            >
                <div className="button-left">
                    <span className="disk-icon">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                            <path d="M22 12h-4l-3 9L9 3l-3 9H2"></path>
                        </svg>
                    </span>
                    <span className="disk-name">
                        {selectedDisk === '_Total' ? 'All Disks' : selectedDisk}
                    </span>
                </div>
                <span className={`chevron ${isOpen ? 'open' : ''}`}>
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <polyline points="6 9 12 15 18 9"></polyline>
                    </svg>
                </span>
            </button>
            
            {isOpen && (
                <div className="disk-dropdown-menu">
                    {availableDisks.map(disk => {
                        const stat = diskStats[disk];
                        
                        return (
                            <div 
                                key={disk} 
                                className={`disk-option ${disk === selectedDisk ? 'selected' : ''}`}
                                onClick={() => handleSelect(disk)}
                            >
                                <span className="option-icon">
                                    {disk === '_Total' ? (
                                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                                            <circle cx="12" cy="12" r="10"></circle>
                                            <line x1="2" y1="12" x2="22" y2="12"></line>
                                            <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path>
                                        </svg>
                                    ) : (
                                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                                            <rect x="2" y="2" width="20" height="8" rx="2" ry="2"></rect>
                                            <rect x="2" y="14" width="20" height="8" rx="2" ry="2"></rect>
                                            <line x1="6" y1="6" x2="6.01" y2="6"></line>
                                            <line x1="6" y1="18" x2="6.01" y2="18"></line>
                                        </svg>
                                    )}
                                </span>
                                <span className="option-label">{disk === '_Total' ? 'All Disks' : disk}</span>
                                
                                {stat && (
                                    <div className="disk-activity-stats">
                                        {stat.read_speed > 0 && <span className="stat-read">R: {formatSpeed(stat.read_speed)}</span>}
                                        {stat.write_speed > 0 && <span className="stat-write">W: {formatSpeed(stat.write_speed)}</span>}
                                    </div>
                                )}

                                {disk === selectedDisk && (
                                    <span className="check-icon">
                                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                                            <polyline points="20 6 9 17 4 12"></polyline>
                                        </svg>
                                    </span>
                                )}
                            </div>
                        );
                    })}
                </div>
            )}
        </div>
    );
};

export default DiskSelector;
