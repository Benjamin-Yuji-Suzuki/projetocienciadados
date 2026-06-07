use polars::prelude::*;
use std::error::Error;
use std::fs;

fn criar_grafico_barras_svg(
    dados: &[(String, f64)],
    titulo: &str,
    subtitulo: &str,
    arquivo: &str,
    cor_grad1: &str, cor_grad2: &str,
) -> Result<(), Box<dyn Error>> {
    let path = format!("graficos/{}", arquivo);
    let w = 900; let h = 520;
    let me = 130; let md = 50; let mt = 80; let mb = 110;
    let pw = w - me - md; let ph = h - mt - mb;

    let max_val = dados.iter().map(|(_, v)| *v).fold(0.0_f64, f64::max).ceil().max(1.0);
    let n = dados.len() as f64;
    let bar_w = (pw as f64 / n).max(18.0);
    let gap = (bar_w * 0.25).max(4.0);

    let mut svg = String::new();
    svg.push_str(&format!(
"<svg xmlns='http://www.w3.org/2000/svg' width='{}' height='{}'>
<defs>
  <linearGradient id='bgg' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#f8f9fa'/><stop offset='100%' stop-color='#e9ecef'/></linearGradient>
  <linearGradient id='bar' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='{}'/><stop offset='100%' stop-color='{}'/></linearGradient>
  <linearGradient id='bar2' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#e63946'/><stop offset='100%' stop-color='#a4161a'/></linearGradient>
  <filter id='sb'><feDropShadow dx='2' dy='3' stdDeviation='4' flood-opacity='0.25'/></filter>
</defs>
<style>text {{ font-family: 'Segoe UI', Arial, sans-serif; }}</style>
<rect width='100%' height='100%' fill='url(#bgg)'/>
<text x='{}' y='35' text-anchor='middle' font-size='22' font-weight='bold' fill='#1a1a2e'>{}</text>
<text x='{}' y='55' text-anchor='middle' font-size='13' fill='#666'>{}</text>",
        w, h, cor_grad1, cor_grad2, w / 2, titulo, w / 2, subtitulo));

    // Eixos
    svg.push_str(&format!("<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#333' stroke-width='1.5'/>", me, mt, me, h - mb));
    svg.push_str(&format!("<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#333' stroke-width='1.5'/>", me, h - mb, w - md, h - mb));

    // Grade horizontal
    for i in 0..=5 {
        let val = max_val * i as f64 / 5.0;
        let y = (h - mb) as f64 - (ph as f64 * i as f64 / 5.0);
        svg.push_str(&format!("<text x='{}' y='{:.0}' text-anchor='end' font-size='12' fill='#888'>{:.1}</text>", me - 10, y + 4.0, val));
        if i > 0 {
            svg.push_str(&format!("<line x1='{}' y1='{:.0}' x2='{}' y2='{:.0}' stroke='#dee2e6' stroke-width='0.5' stroke-dasharray='5,5'/>", me + 1, y, w - md, y));
        }
    }

    // Maior barra usa cor destacada
    let max_idx = dados.iter().enumerate().max_by(|a, b| a.1.1.partial_cmp(&b.1.1).unwrap()).map(|(i, _)| i).unwrap_or(0);

    for (i, (label, valor)) in dados.iter().enumerate() {
        let bar_h = (valor / max_val * ph as f64) as i32;
        let x = me + (i as f64 / n * pw as f64) as i32 + (gap * 0.5) as i32;
        let y = h - mb - bar_h;
        let width = ((bar_w - gap).max(6.0)) as i32;
        let cor = if i == max_idx { "url(#bar2)" } else { "url(#bar)" };

        svg.push_str(&format!("<rect x='{}' y='{}' width='{}' height='{}' fill='{}' rx='4' filter='url(#sb)'/>", x, y, width, bar_h, cor));

        // Rótulo do valor no topo da barra
        svg.push_str(&format!("<text x='{}' y='{}' text-anchor='middle' font-size='12' fill='#333' font-weight='bold'>{:.1}</text>",
            x + width / 2, y - 6, valor));

        // Rótulo do eixo X (rotacionado)
        let lbl = if label.len() > 14 { &label[..14] } else { label };
        svg.push_str(&format!("<text x='{}' y='{}' text-anchor='end' font-size='10' fill='#555' transform='rotate(-40,{},{})'>{}</text>",
            x + width / 2, h - mb + 16, x + width / 2, h - mb + 16, lbl));
    }

    // Legenda
    let ly = h - mb + 65;
    svg.push_str(&format!("<rect x='{}' y='{}' width='180' height='28' fill='white' rx='6' stroke='#ccc'/>", me + 10, ly));
    svg.push_str(&format!("<rect x='{}' y='{}' width='12' height='12' fill='url(#bar)' rx='2'/>", me + 18, ly + 8));
    svg.push_str(&format!("<text x='{}' y='{}' font-size='11' fill='#555'>Demais categorias</text>", me + 36, ly + 18));
    svg.push_str(&format!("<rect x='{}' y='{}' width='12' height='12' fill='url(#bar2)' rx='2'/>", me + 118, ly + 8));
    svg.push_str(&format!("<text x='{}' y='{}' font-size='11' fill='#555'>Maior valor</text>", me + 136, ly + 18));

    // Rótulo do eixo Y
    svg.push_str(&format!("<text x='15' y='{}' text-anchor='middle' font-size='13' fill='#555' transform='rotate(-90,15,{})'>Frequencia</text>", mt + ph / 2, mt + ph / 2));

    svg.push_str("</svg>");
    fs::write(&path, svg)?;
    println!("   Grafico salvo: {}", path);
    Ok(())
}

fn criar_histograma_svg(valores: &[f64], titulo: &str, subtitulo: &str, arquivo: &str) -> Result<(), Box<dyn Error>> {
    let path = format!("graficos/{}", arquivo);
    let w = 900; let h = 500;
    let me = 100; let md = 60; let mt = 80; let mb = 90;
    let pw = w - me - md; let ph = h - mt - mb;
    let n_bins = 25;

    let min_v = valores.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_v = valores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let bw = (max_v - min_v) / n_bins as f64;

    let mut bins = vec![0usize; n_bins];
    for &v in valores {
        if v >= min_v && v < max_v {
            let idx = ((v - min_v) / bw) as usize;
            if idx < n_bins { bins[idx] += 1; }
        }
    }
    let mc = *bins.iter().max().unwrap_or(&1) as f64 * 1.1;

    let mut svg = String::new();
    svg.push_str(&format!(
"<svg xmlns='http://www.w3.org/2000/svg' width='{}' height='{}'>
<defs>
  <linearGradient id='bgg' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#f8f9fa'/><stop offset='100%' stop-color='#e9ecef'/></linearGradient>
  <linearGradient id='hb' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#1e6091'/><stop offset='100%' stop-color='#168aad'/></linearGradient>
  <filter id='sb'><feDropShadow dx='1' dy='2' stdDeviation='3' flood-opacity='0.2'/></filter>
</defs>
<style>text {{ font-family: 'Segoe UI', Arial, sans-serif; }}</style>
<rect width='100%' height='100%' fill='url(#bgg)'/>
<text x='{}' y='35' text-anchor='middle' font-size='22' font-weight='bold' fill='#1a1a2e'>{}</text>
<text x='{}' y='55' text-anchor='middle' font-size='13' fill='#666'>{}</text>
<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#333' stroke-width='1.5'/>
<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#333' stroke-width='1.5'/>",
        w, h, w / 2, titulo, w / 2, subtitulo, me, mt, me, h - mb, me, h - mb, w - md, h - mb));

    for i in 0..=5 {
        let val = mc * i as f64 / 5.0;
        let y = h - mb - (ph as f64 * i as f64 / 5.0) as i32;
        svg.push_str(&format!("<text x='{}' y='{}' text-anchor='end' font-size='11' fill='#888'>{:.0}</text>", me - 8, y + 4, val));
        if i > 0 {
            svg.push_str(&format!("<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#dee2e6' stroke-width='0.5' stroke-dasharray='4,4'/>", me, y, w - md, y));
        }
    }

    // Marcar a mediana com uma linha vertical
    let mut sorted = valores.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_v = sorted[sorted.len() / 2];
    let med_x = me + ((median_v - min_v) / (max_v - min_v).max(1.0) * pw as f64) as i32;

    for (i, &count) in bins.iter().enumerate() {
        let x0 = me + (i as f64 / n_bins as f64 * pw as f64) as i32;
        let x1 = me + ((i + 1) as f64 / n_bins as f64 * pw as f64) as i32;
        let bar_h = (count as f64 / mc * ph as f64) as i32;
        let opacity = 0.5 + 0.5 * (count as f64 / mc);
        svg.push_str(&format!("<rect x='{}' y='{}' width='{}' height='{}' fill='url(#hb)' rx='2' filter='url(#sb)' opacity='{:.2}'/>",
            x0, h - mb - bar_h, (x1 - x0).max(1), bar_h, opacity));
    }

    // Linha da mediana
    svg.push_str(&format!("<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#e63946' stroke-width='2.5' stroke-dasharray='8,4'/>", med_x, mt, med_x, h - mb));
    svg.push_str(&format!("<text x='{}' y='{}' text-anchor='middle' font-size='11' fill='#e63946' font-weight='bold'>Mediana: {:.0}</text>", med_x, mt - 8, median_v));

    // Legenda
    let lx = w - md - 160; let ly = mt + 10;
    svg.push_str(&format!("<rect x='{}' y='{}' width='155' height='40' fill='white' rx='6' stroke='#ccc'/>", lx, ly));
    svg.push_str(&format!("<rect x='{}' y='{}' width='12' height='12' fill='url(#hb)' rx='2'/>", lx + 8, ly + 6));
    svg.push_str(&format!("<text x='{}' y='{}' font-size='11' fill='#555'>Distribuicao (25 bins)</text>", lx + 26, ly + 16));
    svg.push_str(&format!("<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#e63946' stroke-width='2.5' stroke-dasharray='8,4'/>", lx + 8, ly + 30, lx + 20, ly + 30));
    svg.push_str(&format!("<text x='{}' y='{}' font-size='11' fill='#555'>Mediana</text>", lx + 26, ly + 33));

    // Rótulo eixo Y e X
    svg.push_str(&format!("<text x='18' y='{}' text-anchor='middle' font-size='13' fill='#555' transform='rotate(-90,18,{})'>Frequencia</text>", mt + ph / 2, mt + ph / 2));
    svg.push_str(&format!("<text x='{}' y='{}' text-anchor='middle' font-size='13' fill='#555'>total_loss_usd (USD)</text>", me + pw / 2, h - 15));

    svg.push_str("</svg>");
    fs::write(&path, svg)?;
    println!("   Grafico salvo: {}", path);
    Ok(())
}

fn criar_scatter_outliers_svg(valores: &[f64], titulo: &str, subtitulo: &str, arquivo: &str) -> Result<(), Box<dyn Error>> {
    let path = format!("graficos/{}", arquivo);
    let w = 900; let h = 520;
    let me = 100; let md = 60; let mt = 80; let mb = 90;
    let pw = w - me - md; let ph = h - mt - mb;
    let max_v = valores.iter().cloned().fold(f64::NEG_INFINITY, f64::max) * 1.1;
    let n = valores.len();

    let mut sorted = valores.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q1 = sorted[sorted.len() / 4];
    let q3 = sorted[sorted.len() * 3 / 4];
    let iqr = q3 - q1;
    let upper = q3 + 1.5 * iqr;

    // Separar outliers e normais
    let mut outliers: Vec<(usize, f64)> = Vec::new();
    let mut normais: Vec<(usize, f64)> = Vec::new();
    for (i, &v) in valores.iter().enumerate() {
        if v > upper { outliers.push((i, v)); } else { normais.push((i, v)); }
    }

    let mut svg = String::new();
    svg.push_str(&format!(
"<svg xmlns='http://www.w3.org/2000/svg' width='{}' height='{}'>
<defs>
  <linearGradient id='bgg' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#f8f9fa'/><stop offset='100%' stop-color='#e9ecef'/></linearGradient>
  <linearGradient id='norm'><stop offset='0%' stop-color='#457b9d'/><stop offset='100%' stop-color='#1d3557'/></linearGradient>
  <filter id='sb'><feDropShadow dx='1' dy='1' stdDeviation='2' flood-opacity='0.3'/></filter>
</defs>
<style>text {{ font-family: 'Segoe UI', Arial, sans-serif; }}</style>
<rect width='100%' height='100%' fill='url(#bgg)'/>
<text x='{}' y='35' text-anchor='middle' font-size='22' font-weight='bold' fill='#1a1a2e'>{}</text>
<text x='{}' y='55' text-anchor='middle' font-size='13' fill='#666'>{}</text>
<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#333' stroke-width='1.5'/>
<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#333' stroke-width='1.5'/>",
        w, h, w / 2, titulo, w / 2, subtitulo, me, mt, me, h - mb, me, h - mb, w - md, h - mb));

    for i in 0..=5 {
        let val = max_v * i as f64 / 5.0;
        let y = h - mb - (ph as f64 * i as f64 / 5.0) as i32;
        svg.push_str(&format!("<text x='{}' y='{}' text-anchor='end' font-size='11' fill='#888'>{:.0}</text>", me - 8, y + 4, val));
        if i > 0 {
            svg.push_str(&format!("<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#dee2e6' stroke-width='0.5' stroke-dasharray='4,4'/>", me, y, w - md, y));
        }
    }

    // Zona de outliers (area sombreada acima do limite)
    let y_up = h - mb - (upper / max_v * ph as f64) as i32;
    svg.push_str(&format!("<rect x='{}' y='{}' width='{}' height='{}' fill='#e63946' opacity='0.06'/>", me, mt, pw, y_up - mt));
    svg.push_str(&format!("<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#e63946' stroke-dasharray='8,4' stroke-width='2.5'/>", me, y_up, w - md, y_up));
    svg.push_str(&format!("<text x='{}' y='{}' fill='#e63946' font-size='11' font-weight='bold'>Limite IQR: {:.0}</text>", me + 6, y_up - 6, upper));

    // Pontos normais
    for &(ref i, ref v) in &normais {
        let x = me + (*i as f64 / n as f64 * pw as f64) as i32;
        let y = h - mb - (*v / max_v * ph as f64) as i32;
        svg.push_str(&format!("<circle cx='{}' cy='{}' r='3' fill='#457b9d' opacity='0.6'/>", x, y));
    }

    // Outliers em destaque
    for &(ref i, ref v) in &outliers {
        let x = me + (*i as f64 / n as f64 * pw as f64) as i32;
        let y = h - mb - (*v / max_v * ph as f64) as i32;
        svg.push_str(&format!("<circle cx='{}' cy='{}' r='7' fill='none' stroke='#e63946' stroke-width='2' opacity='0.8'/>", x, y));
        svg.push_str(&format!("<circle cx='{}' cy='{}' r='4' fill='#e63946' opacity='0.9' filter='url(#sb)'/>", x, y));
    }

    // Legenda
    let lx = w - md - 155; let ly = mt + 10;
    svg.push_str(&format!("<rect x='{}' y='{}' width='150' height='60' fill='white' rx='6' stroke='#ccc'/>", lx, ly));
    svg.push_str(&format!("<circle cx='{}' cy='{}' r='3' fill='#457b9d'/>", lx + 14, ly + 14));
    svg.push_str(&format!("<text x='{}' y='{}' font-size='11' fill='#555'>Dados normais ({})</text>", lx + 24, ly + 18, normais.len()));
    svg.push_str(&format!("<circle cx='{}' cy='{}' r='4' fill='#e63946' filter='url(#sb)'/>", lx + 14, ly + 38));
    svg.push_str(&format!("<text x='{}' y='{}' font-size='11' fill='#555'>Outliers IQR ({})</text>", lx + 24, ly + 42, outliers.len()));
    svg.push_str(&format!("<rect x='{}' y='{}' width='12' height='2' fill='#e63946'/>", lx + 8, ly + 52));
    svg.push_str(&format!("<text x='{}' y='{}' font-size='9' fill='#888'>Limite superior IQR</text>", lx + 24, ly + 55));

    // Rótulo eixo Y
    svg.push_str(&format!("<text x='18' y='{}' text-anchor='middle' font-size='13' fill='#555' transform='rotate(-90,18,{})'>Prejuizo (USD)</text>", mt + ph / 2, mt + ph / 2));

    svg.push_str("</svg>");
    fs::write(&path, svg)?;
    println!("   Grafico salvo: {}", path);
    Ok(())
}

fn criar_matriz_correlacao_svg(colunas: &[&str], matriz: &[Vec<f64>], titulo: &str, arquivo: &str) -> Result<(), Box<dyn Error>> {
    let path = format!("graficos/{}", arquivo);
    let n = colunas.len();
    let cw = 130; let ch = 48;
    let w = n * cw + 180; let h = n * ch + 150;

    let mut svg = String::new();
    svg.push_str(&format!(
"<svg xmlns='http://www.w3.org/2000/svg' width='{}' height='{}'>
<defs>
  <linearGradient id='bgg' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#f8f9fa'/><stop offset='100%' stop-color='#e9ecef'/></linearGradient>
  <filter id='sb'><feDropShadow dx='1' dy='1' stdDeviation='2' flood-opacity='0.15'/></filter>
</defs>
<style>text {{ font-family: 'Segoe UI', Arial, sans-serif; }}</style>
<rect width='100%' height='100%' fill='url(#bgg)'/>
<text x='{}' y='35' text-anchor='middle' font-size='22' font-weight='bold' fill='#1a1a2e'>{}</text>
<text x='{}' y='55' text-anchor='middle' font-size='13' fill='#666'>Pearson correlation coefficient</text>",
        w, h, w / 2, titulo, w / 2));

    for i in 0..n {
        for j in 0..n {
            let val = matriz[i][j];
            // Paleta divergente: azul (-1) -> branco (0) -> vermelho (+1)
            let (r, g, b) = if val >= 0.0 {
                let intensity = (val * 127.0) as u8;
                (255u8, (255 - intensity * 2).max(128), (255 - intensity * 2).max(128))
            } else {
                let intensity = ((-val) * 127.0) as u8;
                ((255 - intensity * 2).max(128), (255 - intensity * 2).max(128), 255u8)
            };

            let x = 150 + j * cw; let y = 70 + i * ch;
            svg.push_str(&format!("<rect x='{}' y='{}' width='{}' height='{}' fill='rgb({},{},{})' rx='6' filter='url(#sb)'/>", x, y, cw, ch, r, g, b));

            let text_color = if val.abs() > 0.5 { "white" } else { "#333" };
            let font_size = if cw >= 120 { 16 } else { 13 };
            svg.push_str(&format!("<text x='{}' y='{}' text-anchor='middle' dominant-baseline='middle' font-size='{}' fill='{}' font-weight='bold'>{:.2}</text>", x + cw/2, y + ch/2, font_size, text_color, val));
        }
    }

    for (i, c) in colunas.iter().enumerate() {
        let lbl = if c.len() > 18 { &c[..18] } else { c };
        svg.push_str(&format!("<text x='140' y='{}' text-anchor='end' dominant-baseline='middle' font-size='12' fill='#555' font-weight='bold'>{}</text>", 70 + i*ch + ch/2, lbl));
        svg.push_str(&format!("<text x='{}' y='{}' text-anchor='middle' font-size='11' fill='#555'>{}</text>", 150 + i*cw + cw/2, 60, lbl));
    }

    // Barra de cor (color bar) para referencia
    let cb_y = 70 + n * ch + 20;
    svg.push_str(&format!("<text x='{}' y='{}' text-anchor='middle' font-size='11' fill='#555'>Correlacao: </text>", w / 2, cb_y));
    for k in 0..20 {
        let frac = k as f64 / 19.0; // 0..1 map to -1..+1
        let val = frac * 2.0 - 1.0;
        let x = w / 2 - 100 + k * 10;
        let (r, g, b) = if val >= 0.0 {
            let intensity = (val * 127.0) as u8;
            (255u8, (255 - intensity * 2).max(128), (255 - intensity * 2).max(128))
        } else {
            let intensity = ((-val) * 127.0) as u8;
            ((255 - intensity * 2).max(128), (255 - intensity * 2).max(128), 255u8)
        };
        svg.push_str(&format!("<rect x='{}' y='{}' width='10' height='14' fill='rgb({},{},{})' rx='1'/>", x, cb_y + 10, r, g, b));
    }
    svg.push_str(&format!("<text x='{}' y='{}' text-anchor='start' font-size='10' fill='#888'>-1.0</text>", w / 2 - 100, cb_y + 38));
    svg.push_str(&format!("<text x='{}' y='{}' text-anchor='middle' font-size='10' fill='#888'>0.0</text>", w / 2, cb_y + 38));
    svg.push_str(&format!("<text x='{}' y='{}' text-anchor='end' font-size='10' fill='#888'>+1.0</text>", w / 2 + 100, cb_y + 38));

    svg.push_str("</svg>");
    fs::write(&path, svg)?;
    println!("   Grafico salvo: {}", path);
    Ok(())
}

pub fn executar() -> Result<(), Box<dyn Error>> {
    println!(" [EDA] Iniciando Analise Exploratoria...\n");

    fs::create_dir_all("graficos")?;

    let df = LazyFrame::scan_parquet(
        "camada_prata/dataset_ml.parquet",
        ScanArgsParquet::default(),
    )?.collect()?;

    // ====================================================================
    // HIPOTESE 1: "Industrias do setor de tecnologia (codigo 51) sofrem
    //              mais ataques que outros setores"
    // ====================================================================
    println!("H1: Industrias de tecnologia sofrem mais ataques que outros setores?");
    let df_ind = df.clone().lazy()
        .group_by([col("industry_primary")])
        .agg([len().alias("count")])
        .sort(["count"], SortMultipleOptions::default().with_order_descending(true))
        .limit(10)
        .collect()?;

    let x_ind: Vec<String> = df_ind.column("industry_primary")?.str()?
        .into_iter().map(|s| s.unwrap_or("N/A").to_string()).collect();
    let y_ind: Vec<f64> = df_ind.column("count")?
        .cast(&DataType::Float64)?.f64()?
        .into_iter().map(|v| v.unwrap_or(0.0)).collect();

    let dados_ind: Vec<(String, f64)> = x_ind.into_iter().zip(y_ind.into_iter()).collect();
    criar_grafico_barras_svg(&dados_ind,
        "Top 10 Industrias com Maior Volume de Ataques",
        "H1: Setores com mais incidentes de ciberseguranca registrados",
        "grafico1_top_industrias.svg", "#1e6091", "#168aad")?;
    println!("   INTERPRETACAO: Se a industria 51 (technology) ou 52 (finance)");
    println!("   liderar, confirma-se que setores com muitos dados digitais");
    println!("   sao alvos prioritarios.\n");

    // ====================================================================
    // HIPOTESE 2: "Ransomware causa o maior prejuizo financeiro medio"
    // ====================================================================
    println!("H2: Ransomware causa o maior prejuizo financeiro medio?");
    let df_vec = df.clone().lazy()
        .group_by([col("attack_vector_primary")])
        .agg([col("total_loss_usd").mean().alias("avg_loss")])
        .sort(["avg_loss"], SortMultipleOptions::default().with_order_descending(true))
        .limit(10)
        .collect()?;

    let x_vec: Vec<String> = df_vec.column("attack_vector_primary")?.str()?
        .into_iter().map(|s| s.unwrap_or("N/A").to_string()).collect();
    let y_vec: Vec<f64> = df_vec.column("avg_loss")?
        .cast(&DataType::Float64)?.f64()?
        .into_iter().map(|v| v.unwrap_or(0.0)).collect();

    let dados_vec: Vec<(String, f64)> = x_vec.into_iter().zip(y_vec.into_iter()).collect();
    criar_grafico_barras_svg(&dados_vec,
        "Prejuizo Medio por Vetor de Ataque (USD)",
        "H2: Qual tipo de ataque causa maior dano financeiro medio?",
        "grafico2_prejuizo_vetor.svg", "#e63946", "#a4161a")?;
    println!("   INTERPRETACAO: Se ransomware lidera, justifica-se investir");
    println!("   em protecao especifica (backups off-site, treinamento anti-phishing).\n");

    // ====================================================================
    // HIPOTESE 3: "Dados mistos (mixed) sao os mais visados por atacantes"
    // ====================================================================
    println!("H3: Dados mistos (mixed) sao os mais visados por atacantes?");
    let df_data = df.clone().lazy()
        .group_by([col("data_type")])
        .agg([len().alias("count")])
        .sort(["count"], SortMultipleOptions::default().with_order_descending(true))
        .limit(10)
        .collect()?;

    let x_data: Vec<String> = df_data.column("data_type")?.str()?
        .into_iter().map(|s| s.unwrap_or("N/A").to_string()).collect();
    let y_data: Vec<f64> = df_data.column("count")?
        .cast(&DataType::Float64)?.f64()?
        .into_iter().map(|v| v.unwrap_or(0.0)).collect();

    let dados_data: Vec<(String, f64)> = x_data.into_iter().zip(y_data.into_iter()).collect();
    criar_grafico_barras_svg(&dados_data,
        "Frequencia de Tipos de Dados Roubados",
        "H3: Que tipo de dado os atacantes mais visam?",
        "grafico3_tipos_dados.svg", "#2a9d8f", "#1b7a6d")?;
    println!("   INTERPRETACAO: Dados financeiros + pessoais (mixed) sao mais");
    println!("   valiosos no mercado negro. Protecao de dados compostos deve");
    println!("   ser prioridade.\n");

    // ====================================================================
    // GRAFICO 4: Distribuicao de variaveis-chave (total_loss_usd)
    // ====================================================================
    println!("Grafico 4: Distribuicao dos valores de prejuizo total...");
    let perdas: Vec<f64> = df.column("total_loss_usd")?
        .cast(&DataType::Float64)?.f64()?
        .into_iter().map(|v| v.unwrap_or(0.0)).collect();
    criar_histograma_svg(&perdas,
        "Distribuicao do Prejuizo Total (total_loss_usd)",
        "G4: Assimetria a direita - poucos incidentes com perdas muito altas",
        "grafico4_histograma_perdas.svg")?;
    println!("   INTERPRETACAO: A distribuicao e assimetrica a direita (cauda");
    println!("   longa). DECISAO: Usaremos a MEDIANA como referencia para");
    println!("   o target binario (alto/baixo impacto) na modelagem ML.\n");

    // ====================================================================
    // GRAFICO 5: Analise de outliers (metodo IQR)
    // ====================================================================
    println!("Grafico 5: Analise de outliers no prejuizo total...");
    criar_scatter_outliers_svg(&perdas,
        "Outliers no Prejuizo Total (Metodo IQR)",
        "G5: Pontos em vermelho sao outliers acima do limite 1.5x IQR",
        "grafico5_outliers.svg")?;
    println!("   INTERPRETACAO: Pontos acima da linha vermelha sao OUTLIERS.");
    println!("   DECISAO: Na Gold, faremos clipping (capping) no limite");
    println!("   superior do IQR.\n");

    // ====================================================================
    // GRAFICO 6: Matriz de correlacao
    // ====================================================================
    println!("Grafico 6: Matriz de correlacao entre variaveis numericas...");
    let cols_numericas = ["total_loss_usd", "company_revenue_usd", "employee_count", "quality_score"];

    let mut dados_corr: Vec<Vec<f64>> = Vec::new();
    for c in &cols_numericas {
        let serie: Vec<f64> = df.column(c)?.cast(&DataType::Float64)?.f64()?
            .into_iter().map(|v| v.unwrap_or(0.0)).collect();
        dados_corr.push(serie);
    }

    let n = cols_numericas.len();
    let mut matriz = vec![vec![0.0_f64; n]; n];
    for i in 0..n {
        for j in 0..n {
            let xi = &dados_corr[i]; let xj = &dados_corr[j];
            let n_vals = xi.len().min(xj.len());
            let mean_i: f64 = xi.iter().sum::<f64>() / n_vals as f64;
            let mean_j: f64 = xj.iter().sum::<f64>() / n_vals as f64;
            let mut num = 0.0; let mut di_sum = 0.0; let mut dj_sum = 0.0;
            for k in 0..n_vals {
                let di = xi[k] - mean_i; let dj = xj[k] - mean_j;
                num += di * dj; di_sum += di * di; dj_sum += dj * dj;
            }
            let den = (di_sum * dj_sum).sqrt();
            matriz[i][j] = if den > 0.0 { num / den } else { 0.0 };
        }
    }

    criar_matriz_correlacao_svg(&cols_numericas, &matriz,
        "Matriz de Correlacao - Variaveis Numericas",
        "grafico6_matriz_correlacao.svg")?;
    println!("   INTERPRETACAO: Se total_loss_usd tem correlacao alta (>0.5)");
    println!("   com company_revenue_usd, empresas maiores sofrem perdas maiores.");
    println!("   DECISAO: Manteremos revenue e employee_count como features,\n");

    println!("   [EDA] Todos os graficos gerados em 'graficos/'!\n");
    Ok(())
}
