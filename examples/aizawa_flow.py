"""
🌊 AIZAWA ATTRACTOR BENCHMARK: QUAD-VIEW DASHBOARD
=================================================
SymPy vs SymEngine vs SymbAnaFis

Dashboard:
[ SymPy ] [ SymEngine ]
[ SymbAnaFis ] [ Performance ]
"""

import numpy as np
import matplotlib.pyplot as plt
import matplotlib.animation as animation
import matplotlib.gridspec as gridspec
import time
import warnings
warnings.filterwarnings('ignore')

# CONFIGURATION
N_PARTICLES = 500_000
N_STEPS = 600
DT = 0.01

# Aizawa parameters
A, B, C, D, E, F = 0.95, 0.7, 0.6, 3.5, 0.25, 0.1

def setup_sympy():
    try:
        import sympy as sp
        print("\n🔧 SymPy: Setting up...")
        t0 = time.perf_counter()
        
        x, y, z = sp.symbols('x y z')
        dx = (z - B) * x - D * y
        dy = D * x + (z - B) * y
        dz = C + A*z - z**3/3 - (x**2 + y**2) * (1 + E*z) + F * z * x**3
        
        # Lambdify with CSE
        func = sp.lambdify([x, y, z], [dx, dy, dz], 'numpy', cse=True)
        setup_time = time.perf_counter() - t0
        print(f"   ✓ Setup: {setup_time*1000:.1f} ms")
        
        def evaluate(x, y, z):
            return func(x, y, z)
            
        return evaluate, setup_time
    except ImportError: return None, 0

def setup_symengine():
    try:
        import symengine as se
        print("\n🔧 SymEngine: Setting up...")
        t0 = time.perf_counter()
        x, y, z = se.symbols('x y z')
        dx = (z - B) * x - D * y
        dy = D * x + (z - B) * y
        dz = C + A*z - z**3/3 - (x**2 + y**2) * (1 + E*z) + F * z * x**3
        
        try:
            func = se.Lambdify([x, y, z], [dx, dy, dz], backend='llvm')
            print("   (LLVM)")
        except:
            func = se.Lambdify([x, y, z], [dx, dy, dz])
            print("   (Default)")
            
        setup_time = time.perf_counter() - t0
        print(f"   ✓ Setup: {setup_time*1000:.1f} ms")
        
        def evaluate(x, y, z):
            # Stack for SymEngine
            inp = np.column_stack((x, y, z))
            out = func(inp)
            return out[:,0], out[:,1], out[:,2]
            
        return evaluate, setup_time
    except ImportError: return None, 0

def setup_symbanafis():
    try:
        from symb_anafis import parse, eval_f64
        print("\n🔧 SymbAnaFis: Setting up...")
        t0 = time.perf_counter()
        
        # Strings
        ex_dx = f"(z - {B}) * x - {D} * y"
        ex_dy = f"{D} * x + (z - {B}) * y"
        ex_dz = f"{C} + {A}*z - z^3/3 - (x^2 + y^2) * (1 + {E}*z) + {F} * z * x^3"
        
        exprs = [parse(ex_dx), parse(ex_dy), parse(ex_dz)]
        var_names = [["x", "y", "z"]] * 3
        
        setup_time = time.perf_counter() - t0
        print(f"   ✓ Setup: {setup_time*1000:.1f} ms")
        
        def evaluate(x, y, z):
            cols = [x, y, z]
            res = eval_f64(exprs, var_names, [cols, cols, cols])
            return res[0], res[1], res[2]
            
        return evaluate, setup_time
    except ImportError: return None, 0

def main():
    print(f"🌊 AIZAWA QUAD-VIEW: {N_PARTICLES:,} Particles")
    
    # Init
    x0 = np.random.normal(0, 0.1, N_PARTICLES)
    y0 = np.random.normal(0, 0.1, N_PARTICLES)
    z0 = np.random.normal(0, 0.5, N_PARTICLES)
    
    engines = [
        ('SymPy', setup_sympy, '#60a5fa'),
        ('SymEngine', setup_symengine, '#34d399'),
        ('SymbAnaFis', setup_symbanafis, '#fbbf24'),
    ]
    
    results = []
    
    # Vis subset
    vis_count = 10000 # Reduced from 50k to 10k for faster video generation (Matplotlib 3D is slow)
    vis_idx = np.linspace(0, N_PARTICLES-1, vis_count, dtype=int)
    
    for name, setup, color in engines:
        fn, s_time = setup()
        if not fn: continue
        
        # Warmup
        try: fn(x0[:100], y0[:100], z0[:100])
        except: continue
        
        print(f"   Running {name}...")
        
        x, y, z = x0.copy(), y0.copy(), z0.copy()
        
        # History
        hx = np.zeros((N_STEPS, vis_count))
        hy = np.zeros((N_STEPS, vis_count))
        hz = np.zeros((N_STEPS, vis_count))
        
        t0 = time.perf_counter()
        
        # Euler Integration (Steps)
        dt = DT
        for i in range(N_STEPS):
            hx[i], hy[i], hz[i] = x[vis_idx], y[vis_idx], z[vis_idx]
            
            dx, dy, dz = fn(x, y, z)
            # x += dx * dt
            x += dx * dt
            y += dy * dt
            z += dz * dt
            
        run_time = time.perf_counter() - t0
        print(f"   ✓ Time: {run_time:.4f}s")
        
        results.append({
            "name": name, "run_time": run_time, "color": color,
            "hx": hx, "hy": hy, "hz": hz
        })
        
    if not results: return

    # --- QUAD-VIEW VIDEO ---
    print("\n🎥 Generating Quad-View Video...")
    
    fig = plt.figure(figsize=(16, 9), facecolor='#0f172a')
    gs = gridspec.GridSpec(2, 2, width_ratios=[1, 1], height_ratios=[1, 1],
                           wspace=0.1, hspace=0.15, left=0.05, right=0.95, top=0.9, bottom=0.05)
                           
    axes_map = {0: gs[0, 0], 1: gs[0, 1], 2: gs[1, 0]}
    
    subplots = []
    
    for i in range(3):
        if i >= len(results): break
        res = results[i]
        
        ax = fig.add_subplot(axes_map[i], projection='3d', facecolor='black')
        ax.set_facecolor('#0f172a')
        
        ax.set_xlim(-2, 2)
        ax.set_ylim(-2, 2)
        ax.set_zlim(-1, 3)
        ax.set_axis_off()
        ax.set_title(f"{res['name']}", color=res['color'], fontsize=14, fontweight='bold')
        
        scat = ax.scatter([], [], [], s=1, c=res['color'], alpha=0.1)
        subplots.append({'scat': scat, 'hx': res['hx'], 'hy': res['hy'], 'hz': res['hz'], 'ax': ax})
        
    # Performance
    ax_perf = fig.add_subplot(gs[1, 1], facecolor='#1e293b')
    names = [r['name'] for r in results]
    times = [r['run_time'] for r in results]
    colors = [r['color'] for r in results]
    y = np.arange(len(names))
    
    ax_perf.barh(y, times, color=colors)
    ax_perf.set_yticks(y)
    ax_perf.set_yticklabels(names, color='white', fontweight='bold')
    ax_perf.invert_yaxis()
    ax_perf.set_xlabel('Time (s)', color='#94a3b8')
    ax_perf.set_title("Performance", color='white')
    ax_perf.spines['right'].set_visible(False)
    ax_perf.spines['top'].set_visible(False)
    ax_perf.spines['left'].set_visible(False)
    ax_perf.spines['bottom'].set_color('#475569')
    ax_perf.tick_params(colors='#94a3b8')
    
    for i, v in enumerate(times):
        ax_perf.text(v, i, f" {v:.4f}s", color='white', va='center')
        
    fig.suptitle(f"Aizawa Attractor: {N_PARTICLES:,} Particles", 
                 color='white', fontsize=20, fontweight='bold', y=0.98)
                 
    def update(frame):
        for sub in subplots:
            frame_idx = min(frame, N_STEPS-1)
            x = sub['hx'][frame_idx]
            y = sub['hy'][frame_idx]
            z = sub['hz'][frame_idx]
            
            sub['scat']._offsets3d = (x, y, z)
            sub['ax'].view_init(elev=30, azim=frame)
            
        return []
        
    ani = animation.FuncAnimation(fig, update, frames=N_STEPS, interval=40, blit=False)
    
    try:
        from matplotlib.animation import FFMpegWriter
        from pathlib import Path
        video_dir = Path(__file__).parent.parent / 'videos'
        video_dir.mkdir(exist_ok=True)
        out_path = video_dir / 'aizawa_quad.mp4'
        writer = FFMpegWriter(fps=30, codec='h264_nvenc', 
                              extra_args=['-preset', 'fast', '-rc', 'vbr', '-cq', '26'])
        ani.save(str(out_path), writer=writer, dpi=100)
        print(f"✨ Saved {out_path} (GPU)")
    except:
        from pathlib import Path
        video_dir = Path(__file__).parent.parent / 'videos'
        video_dir.mkdir(exist_ok=True)
        out_path = video_dir / 'aizawa_quad.mp4'
        ani.save(str(out_path), fps=30, dpi=100)
        print(f"✨ Saved {out_path} (CPU)")

if __name__ == "__main__":
    main()
