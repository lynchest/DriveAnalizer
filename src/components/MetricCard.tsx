import React from 'react';
import clsx from 'clsx';
import './MetricCard.css';

interface MetricCardProps {
    label: string;
    value: string | number;
    unit: string;
    variant?: 'default' | 'primary' | 'secondary' | 'accent' | 'success' | 'warning' | 'danger';
}

const MetricCard: React.FC<MetricCardProps> = ({ label, value, unit, variant = 'default' }) => {
    return (
        <div className={clsx('metric-card', `metric-card--${variant}`)}>
            <div className="metric-card__header">
                <span className="metric-card__label">{label}</span>
            </div>
            <div className="metric-card__body">
                <span className="metric-card__value">{value}</span>
                <span className="metric-card__unit">{unit}</span>
            </div>
        </div>
    );
};

export default MetricCard;
