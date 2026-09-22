import { memo, useEffect, useState } from 'react';

const COLORS = ['#e53935', '#fdd835', '#4caf50', '#2196f8', '#ff9800', '#9c27b0', '#00bcd4'];
const COUNT = 90;

const rand = (min, max) => Math.random() * (max - min) + min;

const Confetti = memo(function Confetti({ active }) {
  const [particles, setParticles] = useState([]);

  useEffect(() => {
    if (!active) {
      setParticles([]);
      return;
    }
    const arr = [];
    for (let i = 0; i < COUNT; i++) {
      const drift = (i % 2 === 0 ? 1 : -1) * rand(30, 90);
      arr.push({
        id: i + Math.random().toString(16).slice(2),
        left: rand(0, 100),
        size: rand(6, 12),
        color: COLORS[Math.floor(Math.random() * COLORS.length)],
        delay: rand(0, 1.2),
        duration: rand(2.2, 3.6),
        rotate: rand(0, 360),
        drift,
      });
    }
    setParticles(arr);
  }, [active]);

  if (!active || particles.length === 0) return null;

  return (
    <div className="confetti-overlay" aria-hidden="true">
      {particles.map((p) => (
        <div
          key={p.id}
          className="confetti-piece"
          style={{
            left: `${p.left}%`,
            width: `${p.size}px`,
            height: `${p.size}px`,
            backgroundColor: p.color,
            animationDelay: `${p.delay}s`,
            animationDuration: `${p.duration}s`,
            '--rotate': `${p.rotate}deg`,
            '--drift': `${p.drift}px`,
          }}
        />
      ))}
    </div>
  );
});

export default Confetti;
