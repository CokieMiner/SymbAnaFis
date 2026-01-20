"""
🌀 CLIFFORD ATTRACTOR BENCHMARK: QUAD-VIEW DASHBOARD
====================================================
SymPy vs SymEngine vs SymbAnaFis

Dashboard:
[ SymPy ] [ SymEngine ]
[ SymbAnaFis ] [ Performance ]
"""

import time
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.animation as animation
import matplotlib.gridspec as gridspec
import warnings
warnings.filterwarnings('ignore')

# CONFIGURATION
N_PARTICLES = 1_000_000 
N_STEPS = 50
INTERP_FRAMES = 20 # Frames between steps to smooth/slow motion
A, B, C, D = -1.4, 1.6, 1.0, 0.7

# EQUATIONS
EXPR_X = "sin(a*y) + c*cos(a*x)"
EXPR_Y = "sin(b*x) + d*cos(b*y)"

# ================= ENGINES =================

def setup_sympy():
    try:
        import sympy as sp
        print("🔧 SymPy: Setting up...")
        t0 = time.perf_counter()
        
        x, y, a, b, c, d = sp.symbols('x y a b c d')
        eq_x = sp.sin(a*y) + c*sp.cos(a*x)
        eq_y = sp.sin(b*x) + d*sp.cos(b*y)
        
        subs = {a: A, b: B, c: C, d: D}
        eq_x = eq_x.subs(subs)
        eq_y = eq_y.subs(subs)
        
        func = sp.lambdify([x, y], [eq_x, eq_y], 'numpy')
        
        setup_time = time.perf_counter() - t0
        print(f"   ✓ Setup: {setup_time*1000:.1f} ms")
        
        def evaluate(x_arr, y_arr):
            return func(x_arr, y_arr)
            
        return evaluate, setup_time
    except ImportError: return None, 0.0

def setup_symengine():
    try:
        import symengine as se
        print("🔧 SymEngine: Setting up...")
        t0 = time.perf_counter()
        
        x, y, a, b, c, d = se.symbols('x y a b c d')
        eq_x = se.sin(a*y) + c*se.cos(a*x)
        eq_y = se.sin(b*x) + d*se.cos(b*y)
        
        subs = {a: A, b: B, c: C, d: D}
        eq_x = eq_x.subs(subs)
        eq_y = eq_y.subs(subs)
        
        try:
            func = se.Lambdify([x, y], [eq_x, eq_y], backend='llvm')
            print("   (Backend: LLVM)")
        except:
            func = se.Lambdify([x, y], [eq_x, eq_y], backend='lambda_double')
            print("   (Backend: C++ Lambda)")
            
        setup_time = time.perf_counter() - t0
        print(f"   ✓ Setup: {setup_time*1000:.1f} ms")
        
        def evaluate(x_arr, y_arr):
            inp = np.column_stack((x_arr, y_arr))
            out = func(inp)
            return out[:, 0], out[:, 1]
            
        return evaluate, setup_time
    except ImportError: return None, 0.0

def setup_symbanafis_par():
    try:
        from symb_anafis import parse, eval_f64
        print("🔧 SymbAnaFis (Par): Setting up...")
        t0 = time.perf_counter()
        
        e_x = f"sin({A}*y) + {C}*cos({A}*x)"
        e_y = f"sin({B}*x) + {D}*cos({B}*y)"
        
        exprs = [parse(e_x), parse(e_y)]
        var_names = [["x", "y"], ["x", "y"]]
        
        setup_time = time.perf_counter() - t0
        print(f"   ✓ Setup: {setup_time*1000:.3f} ms")
        
        def evaluate(x_arr, y_arr):
            # Zero-copy
            cols = [x_arr, y_arr]
            res = eval_f64(exprs, var_names, [cols, cols])
            return res[0], res[1]
        
        return evaluate, setup_time
    except ImportError as e: 
        print(f"   ⚠ SymbAnaFis Import Error: {e}")
        return None, 0.0

# ================= MAIN =================

def main():
    print("🌀 CLIFFORD ATTRACTOR QUAD-VIEW")
    
    # Init
    initial_x = np.random.uniform(-2, 2, N_PARTICLES)
    initial_y = np.random.uniform(-2, 2, N_PARTICLES)
    
    engines = [
        ('SymPy', setup_sympy, '#60a5fa'),
        ('SymEngine', setup_symengine, '#34d399'),
        ('SymbAnaFis', setup_symbanafis_par, '#fbbf24'),
    ]
    
    results = []
    
    vis_count = 100000 # Higher for 2D scatter
    step_idx = np.linspace(0, N_PARTICLES-1, vis_count, dtype=int)
    
    for name, setup, color in engines:
        fn, s_time = setup()
        if not fn: continue
        
        # Warmup
        try:
            fn(initial_x[:100], initial_y[:100])
        except: continue
        
        print(f"   Running {name}...")
        
        x = initial_x.copy()
        y = initial_y.copy()
        
        # Store Frames for animation
        # Since Clifford evolves fast, we can save every frame
        # Only save subset
        history_x = np.zeros((N_STEPS, vis_count))
        history_y = np.zeros((N_STEPS, vis_count))
        
        start = time.perf_counter()
        for i in range(N_STEPS):
            history_x[i] = x[step_idx]
            history_y[i] = y[step_idx]
            x, y = fn(x, y) # Pointer update
            
        run_time = time.perf_counter() - start
        print(f"   ✓ Time: {run_time:.4f}s")
        
        results.append({
            "name": name,
            "run_time": run_time,
            "color": color,
            "hist_x": history_x,
            "hist_y": history_y
        })
        
    if not results: return

    # --- QUAD-VIEW VIDEO ---
    print("\n🎥 Generating Quad-View Video...")
    
    fig = plt.figure(figsize=(16, 9), facecolor='#0f172a')
    gs = gridspec.GridSpec(2, 2, width_ratios=[1, 1], height_ratios=[1, 1],
                           wspace=0.1, hspace=0.15, left=0.05, right=0.95, top=0.9, bottom=0.05)
    
    axes_map = {0: gs[0, 0], 1: gs[0, 1], 2: gs[1, 0]} # BL for 3rd
    
    subplots = []
    
    for i in range(3):
        if i >= len(results): break
        res = results[i]
        
        ax = fig.add_subplot(axes_map[i], facecolor='black')
        ax.set_facecolor('#0f172a')
        ax.set_xlim(-2.5, 2.5)
        ax.set_ylim(-2.5, 2.5)
        ax.axis('off')
        ax.set_title(f"{res['name']}", color=res['color'], fontsize=14, fontweight='bold')
        
        scat = ax.scatter([], [], s=0.2, c=res['color'], alpha=0.6)
        subplots.append({'scat': scat, 'hx': res['hist_x'], 'hy': res['hist_y']})
        
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
    ax_perf.set_title("Calculation Time", color='white')
    ax_perf.spines['right'].set_visible(False)
    ax_perf.spines['top'].set_visible(False)
    ax_perf.spines['left'].set_visible(False)
    ax_perf.spines['bottom'].set_color('#475569')
    ax_perf.tick_params(colors='#94a3b8')
    
    for i, v in enumerate(times):
        ax_perf.text(v, i, f" {v:.4f}s", color='white', va='center')
        
    fig.suptitle(f"Clifford Attractor: {N_PARTICLES:,} Particles", 
                 color='white', fontsize=20, fontweight='bold', y=0.98)
                 
    total_frames = (N_STEPS - 1) * INTERP_FRAMES
                 
    def update(frame):
        # Calculate step index and interpolation factor alpha
        step_idx = frame // INTERP_FRAMES
        alpha = (frame % INTERP_FRAMES) / INTERP_FRAMES
        
        # Next step
        next_idx = step_idx + 1
        if next_idx >= N_STEPS: 
            next_idx = step_idx
            alpha = 0
            
        for sub in subplots:
            # Current and Next positions
            x0, y0 = sub['hx'][step_idx], sub['hy'][step_idx]
            x1, y1 = sub['hx'][next_idx], sub['hy'][next_idx]
            
            # Linear Interpolation
            # x_vis = x0 * (1-a) + x1 * a
            # Creates smooth transition "slow motion" effect
            x_vis = x0 * (1 - alpha) + x1 * alpha
            y_vis = y0 * (1 - alpha) + y1 * alpha
            
            sub['scat'].set_offsets(np.column_stack((x_vis, y_vis)))
            # increase alpha over time
            sub['scat'].set_alpha(0.1 + 0.8 * (step_idx/N_STEPS))
        return []
        
    ani = animation.FuncAnimation(fig, update, frames=total_frames, interval=30, blit=False)
    
    try:
        from matplotlib.animation import FFMpegWriter
        from pathlib import Path
        video_dir = Path(__file__).parent.parent / 'videos'
        video_dir.mkdir(exist_ok=True)
        out_path = video_dir / 'clifford_quad.mp4'
        
        # Normal FPS again
        writer = FFMpegWriter(fps=30, codec='h264_nvenc', 
                              extra_args=['-preset', 'fast', '-rc', 'vbr', '-cq', '26'])
        ani.save(str(out_path), writer=writer, dpi=100)
        print(f"✨ Saved {out_path} (GPU)")
    except:
        from pathlib import Path
        video_dir = Path(__file__).parent.parent / 'videos'
        video_dir.mkdir(exist_ok=True)
        out_path = video_dir / 'clifford_quad.mp4'
        ani.save(str(out_path), fps=30, dpi=100)
        print(f"✨ Saved {out_path} (CPU)")

if __name__ == "__main__":
    main()
