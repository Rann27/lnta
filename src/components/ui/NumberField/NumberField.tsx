import React, { useState, useEffect } from "react";
import { formatNumber, parseNumber } from "../../../lib/format";
import { TextField } from "../TextField";

export interface NumberFieldProps {
  label?: string;
  value?: number;
  onChange?: (value: number | undefined) => void;
  min?: number;
  max?: number;
  step?: number;
  unit?: string;
  error?: string | boolean;
  hint?: string;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
}

export const NumberField: React.FC<NumberFieldProps> = ({
  label,
  value,
  onChange,
  min,
  max,
  step = 1,
  unit,
  error,
  hint,
  placeholder,
  disabled,
  className,
}) => {
  const [isFocused, setIsFocused] = useState(false);
  const [textValue, setTextValue] = useState<string>(() =>
    value !== undefined ? formatNumber(value) : "",
  );

  useEffect(() => {
    if (!isFocused) {
      setTextValue(value !== undefined ? formatNumber(value) : "");
    }
  }, [value, isFocused]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const raw = e.target.value;
    setTextValue(raw);

    // Grouping separators differ per locale ("1.500" is 1500 in Indonesian).
    onChange?.(parseNumber(raw));
  };

  const handleFocus = () => {
    setIsFocused(true);
    setTextValue(value !== undefined ? String(value) : "");
  };

  const handleBlur = () => {
    setIsFocused(false);
    if (value !== undefined) {
      // Clamp if min/max provided
      let clamped = value;
      if (min !== undefined && clamped < min) clamped = min;
      if (max !== undefined && clamped > max) clamped = max;
      if (clamped !== value) {
        onChange?.(clamped);
      }
      setTextValue(formatNumber(clamped));
    } else {
      setTextValue("");
    }
  };

  return (
    <TextField
      inputMode="numeric"
      step={step}
      label={label}
      value={textValue}
      onChange={handleChange}
      onFocus={handleFocus}
      onBlur={handleBlur}
      placeholder={placeholder}
      error={error}
      hint={hint}
      disabled={disabled}
      className={className}
      rightElement={unit ? <span>{unit}</span> : undefined}
    />
  );
};
