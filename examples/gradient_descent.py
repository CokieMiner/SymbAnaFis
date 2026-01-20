"""
🏔️ GRADIENT DESCENT: QUAD-VIEW AVALANCHE 🏔️
=============================================
SymPy vs SymEngine vs SymbAnaFis

Quad-View Dashboard:
[ SymPy 3D ] [ SymEngine 3D ]
[ SymbAnaFis 3D ] [ Performance Chart ]
"""

import numpy as np
import matplotlib.pyplot as plt
import matplotlib.animation as animation
import matplotlib.gridspec as gridspec
import time
import warnings
warnings.filterwarnings('ignore')

# ================= CONFIGURATION =================
# Scale: 500,000 Particles
N_BALLS = 500_000 
STEPS = 600      
LR = 0.05
LIMIT = 8.0 

# Complex terrain h(x,y)
EXPR_STR = "sin(x) * cos(y) + 0.1 * (x^2 + y^2) - 0.2 * cos(1.5 * x - 1.0) * sin(2.0 * y) + 0.2 * erf(x/2.0) * erf(y/2.0)"

def terrain_z(x, y):
    from scipy.special import erf
    return np.sin(x)*np.cos(y) + 0.1*(x**2+y**2) - 0.2*np.cos(1.5*x - 1.0)*np.sin(2.0*y) + 0.2*erf(x/2.0)*erf(y/2.0)

# ================= ENGINES =================

def setup_sympy():
    # Force SymPy - No try/except to hide it
    import sympy as sp
    # from sympy.special.error_functions import erf
    print("\n🔧 SymPy: Setting up...")
    t0 = time.perf_counter()
    
    x, y = sp.symbols('x y')
    h = sp.sin(x) * sp.cos(y) + 0.1 * (x**2 + y**2) - 0.2 * sp.cos(1.5*x - 1.0) * sp.sin(2.0*y) + 0.2 * sp.erf(x/2.0) * sp.erf(y/2.0)
    
    print(f"   Deriving...")
    diff_t0 = time.perf_counter()
    dh_dx = sp.diff(h, x)
    dh_dy = sp.diff(h, y)
    diff_time = time.perf_counter() - diff_t0
    
    print(f"   Compiling...")
    comp_t0 = time.perf_counter()
    import scipy
    # CSE=True helps SymPy performance
    # modules=['numpy', 'scipy'] allows erf to be mapped to scipy.special.erf correctly
    grad_fn = sp.lambdify([x, y], [dh_dx, dh_dy], modules=['numpy', 'scipy'], cse=True)
    comp_time = time.perf_counter() - comp_t0
    
    setup_time = time.perf_counter() - t0
    print(f"   ✓ Diff: {diff_time*1000:.2f}ms | Compile: {comp_time*1000:.2f}ms")
    
    def evaluate(x_arr, y_arr):
        dx, dy = grad_fn(x_arr, y_arr)
        return dx, dy
        
    return evaluate, diff_time, comp_time


def setup_symengine():
    try:
        import symengine as se
        print("\n🔧 SymEngine: Setting up...")
        t0 = time.perf_counter()
        
        x, y = se.symbols('x y')
        h = se.sin(x) * se.cos(y) + 0.1 * (x**2 + y**2) - 0.2 * se.cos(1.5*x - 1.0) * se.sin(2.0*y) + 0.2 * se.erf(x/2.0) * se.erf(y/2.0)
        
        print(f"   Deriving...")
        diff_t0 = time.perf_counter()
        dh_dx = h.diff(x)
        dh_dy = h.diff(y)
        diff_time = time.perf_counter() - diff_t0
        
        print(f"   Compiling...")
        comp_t0 = time.perf_counter()
        try:
            grad_fn_inner = se.Lambdify([x, y], [dh_dx, dh_dy], backend='llvm')
            print("   (LLVM Backend)")
            def evaluate(x_arr, y_arr):
                inp = np.column_stack((x_arr, y_arr))
                out = grad_fn_inner(inp)
                return out[:,0], out[:,1]
        except:
            grad_fn_inner = se.Lambdify([x, y], [dh_dx, dh_dy])
            print("   (Default Backend)")
            def evaluate(x_arr, y_arr):
                inp = np.column_stack((x_arr, y_arr))
                out = grad_fn_inner(inp)
                return out[:,0], out[:,1]
                
        comp_time = time.perf_counter() - comp_t0
        setup_time = time.perf_counter() - t0
        print(f"   ✓ Diff: {diff_time*1000:.2f}ms | Compile: {comp_time*1000:.2f}ms")
        return evaluate, diff_time, comp_time
    except ImportError:
        print("   ❌ SymEngine not found!")
        return None, 0, 0

def setup_symbanafis():
    try:
        from symb_anafis import parse, eval_f64
        print("\n🔧 SymbAnaFis: Setting up...")
        t0 = time.perf_counter()
        
        h = parse(EXPR_STR)
        
        print(f"   Deriving...")
        diff_t0 = time.perf_counter()
        dh_dx = h.diff("x")
        dh_dy = h.diff("y")
        diff_time = time.perf_counter() - diff_t0
        
        # Lazy compile
        exprs = [dh_dx, dh_dy]
        var_names = [["x", "y"], ["x", "y"]]
        comp_time = 0.0 
        
        setup_time = time.perf_counter() - t0
        print(f"   ✓ Diff: {diff_time*1000:.2f}ms | Compile: ~0ms (Lazy)")
        
        def evaluate(x_arr, y_arr):
            data = [[x_arr, y_arr], [x_arr, y_arr]]
            res = eval_f64(exprs, var_names, data)
            return np.array(res[0]), np.array(res[1])
            
        return evaluate, diff_time, comp_time
    except ImportError:
        print("   ❌ SymbAnaFis not found!")
        return None, 0, 0

# ================= SIMULATION =================

def simulate(name, eval_fn, x0, y0):
    # Visualization subset
    vis_count = 2000
    step = max(1, N_BALLS // vis_count)
    vis_indices = np.arange(0, N_BALLS, step)[:vis_count] 
    
    history = np.zeros((STEPS, len(vis_indices), 2))
    
    x = x0.copy()
    y = y0.copy()
    
    t0 = time.perf_counter()
    for i in range(STEPS):
        history[i] = np.column_stack((x[vis_indices], y[vis_indices]))
        
        gx, gy = eval_fn(x, y)
        
        np.multiply(gx, LR, out=gx)
        np.subtract(x, gx, out=x)
        
        np.multiply(gy, LR, out=gy)
        np.subtract(y, gy, out=y)
        
    run_time = time.perf_counter() - t0
    return run_time, history

# ================= MAIN =================

def main():
    print(f"🏔️ GRADIENT DESCENT: QUAD-VIEW AVALANCHE")
    print(f"   Particles: {N_BALLS:,}")
    print(f"   Steps:     {STEPS}")
    print("-" * 60)

    np.random.seed(42)
    # Gaussian Groups
    n_a = N_BALLS // 2
    xa = np.random.normal(-2.5, 0.8, n_a)
    ya = np.random.normal(2.5, 0.8, n_a)
    n_b = N_BALLS - n_a
    xb = np.random.normal(2.5, 0.8, n_b)
    yb = np.random.normal(-2.5, 0.8, n_b)
    x0 = np.concatenate([xa, xb])
    y0 = np.concatenate([ya, yb])
    
    # Define Engines
    engines = [
        ("SymPy", setup_sympy, "#60a5fa"),      # Blue
        ("SymEngine", setup_symengine, "#34d399"), # Green
        ("SymbAnaFis", setup_symbanafis, "#fbbf24"), # Amber/Gold
    ]
    
    results = []
    
    for name, setup_fn, color in engines:
        try:
            eval_fn, dt, ct = setup_fn()
            if eval_fn is None:
                print(f"Skipping {name} (Setup failed)")
                continue
                
            print(f"   Warming up {name}...")
            eval_fn(x0[:100], y0[:100])
            
            print(f"   🚀 Running Simulation ({N_BALLS:,} particles)...")
            run_time, hist = simulate(name, eval_fn, x0, y0)
            print(f"   ✓ Run Time: {run_time:.4f}s")
            
            results.append({
                "name": name,
                "diff_time": dt,
                "comp_time": ct,
                "run_time": run_time,
                "total_time": dt + ct + run_time,
                "history": hist,
                "color": color
            })
        except Exception as e:
            print(f"   ❌ {name} Failed: {e}")

    if not results:
        print("No results to plot.")
        return

    # --- QUAD-VIEW VIDEO ---
    print(f"\n🎥 Generating Quad-View Video...")

    fig = plt.figure(figsize=(16, 9), facecolor='#0f172a')
    gs = gridspec.GridSpec(2, 2, width_ratios=[1, 1], height_ratios=[1, 1], 
                           wspace=0.1, hspace=0.15, left=0.05, right=0.95, top=0.9, bottom=0.05)
    
    # Mappings for subplot positions
    # Top-Left, Top-Right, Bottom-Left, Bottom-Right
    axes_map = {
        0: gs[0, 0], # Top-Left
        1: gs[0, 1], # Top-Right
        2: gs[1, 0]  # Bottom-Left
    }
    
    subplots = []
    
    # Setup 3D Subplots for Simulations
    low, high = -LIMIT, LIMIT
    X = np.linspace(low, high, 80) # Lower res for quad view to be faster
    Y = np.linspace(low, high, 80)
    XX, YY = np.meshgrid(X, Y)
    ZZ = terrain_z(XX, YY)
    
    for i in range(3):
        if i >= len(results):
            break
            
        res = results[i]
        pos = axes_map[i]
        
        ax = fig.add_subplot(pos, projection='3d', facecolor='black')
        ax.set_facecolor('#0f172a') 
        
        # Terrain
        ax.plot_surface(XX, YY, ZZ, cmap='viridis', alpha=0.3, 
                        rstride=4, cstride=4, linewidth=0, antialiased=False)
        ax.plot_wireframe(XX, YY, ZZ, color='cyan', alpha=0.05, rstride=8, cstride=8)
        
        # Particles
        hist = res['history']
        pts_x = hist[0][:, 0]
        pts_y = hist[0][:, 1]
        pts_z = terrain_z(pts_x, pts_y) + 0.1
        
        scat = ax.scatter(pts_x, pts_y, pts_z, c=res['color'], s=10, alpha=0.8, depthshade=False)
        
        ax.set_xlim(low, high)
        ax.set_ylim(low, high)
        ax.set_zlim(-3, 5)
        ax.set_axis_off()
        ax.view_init(elev=70, azim=-45)
        
        ax.set_title(f"{res['name']}", color=res['color'], fontsize=14, fontweight='bold')
        
        subplots.append({'ax': ax, 'scat': scat, 'hist': hist})

    # 4. Performance Chart (Bottom-Right)
    ax_perf = fig.add_subplot(gs[1, 1], facecolor='#1e293b')
    
    names = [r['name'] for r in results]
    r_times = [r['run_time'] for r in results]
    d_times = [r['diff_time'] + r['comp_time'] for r in results]
    colors = [r['color'] for r in results]
    y_pos = np.arange(len(names))
    
    # Stacked horizontal bars
    ax_perf.barh(y_pos, d_times, align='center', color='gray', alpha=0.5, label="Prep")
    ax_perf.barh(y_pos, r_times, left=d_times, align='center', color=colors, alpha=0.9, label="Run")
    
    ax_perf.set_yticks(y_pos)
    ax_perf.set_yticklabels(names, color='white', fontweight='bold', fontsize=12)
    ax_perf.invert_yaxis()
    ax_perf.set_xlabel('Time (s)', color='#94a3b8')
    ax_perf.set_title("Performance Comparison", color='white', fontweight='bold')
    ax_perf.tick_params(colors='#94a3b8')
    
    # Remove borders
    ax_perf.spines['top'].set_visible(False)
    ax_perf.spines['right'].set_visible(False)
    ax_perf.spines['bottom'].set_color('#475569')
    ax_perf.spines['left'].set_visible(False)
    
    # Labels
    for i, (d, r) in enumerate(zip(d_times, r_times)):
        total = d + r
        ax_perf.text(total + max(d+r for d,r in zip(d_times,r_times))*0.02, i, 
                     f"{total:.3f}s", color='white', va='center', fontweight='bold')

    # Main Title
    fig.suptitle(f"Quad-View Benchmark: {N_BALLS:,} Particles", 
                 color='white', fontsize=20, fontweight='bold', y=0.98)

    # Frame Update
    def update(frame):
        for sub in subplots:
            hist = sub['hist']
            scat = sub['scat']
            # Loop video if steps differ? Assumed same STEPS
            idx = min(frame, len(hist)-1)
            
            hx = hist[idx][:, 0]
            hy = hist[idx][:, 1]
            hz = terrain_z(hx, hy) + 0.1
            
            scat._offsets3d = (hx, hy, hz)
            
            # Rotation
            sub['ax'].view_init(elev=70, azim=-45 + frame*0.1)
            
        return [s['scat'] for s in subplots]
        
    ani = animation.FuncAnimation(fig, update, frames=STEPS, interval=30, blit=False)
    
    try:
        from matplotlib.animation import FFMpegWriter
        from pathlib import Path
        video_dir = Path(__file__).parent.parent / 'videos'
        video_dir.mkdir(exist_ok=True)
        out_path = video_dir / 'gradient_descent_quad.mp4'
        writer = FFMpegWriter(fps=30, codec='h264_nvenc', 
                              extra_args=['-preset', 'fast', '-rc', 'vbr', '-cq', '24'])
        ani.save(str(out_path), writer=writer, dpi=100)
        print(f"✨ Saved {out_path} (GPU)")
    except Exception as e:
        from pathlib import Path
        video_dir = Path(__file__).parent.parent / 'videos'
        video_dir.mkdir(exist_ok=True)
        out_path = video_dir / 'gradient_descent_quad.mp4'
        print(f"   GPU encoding failed ({e}), using CPU...")
        ani.save(str(out_path), fps=30, dpi=100)
        print(f"✨ Saved {out_path} (CPU)")

if __name__ == "__main__":
    main()
