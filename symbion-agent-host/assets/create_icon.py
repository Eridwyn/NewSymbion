#!/usr/bin/env python3
"""
Crée une icône PNG simple pour le system tray Symbion
"""
try:
    from PIL import Image, ImageDraw, ImageFont
    import io
    
    # Créer une image 32x32 avec fond transparent
    size = 32
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    # Dessiner un cercle bleu avec un "S" pour Symbion
    # Fond cercle bleu
    draw.ellipse([2, 2, size-2, size-2], fill=(59, 130, 246, 255), outline=(29, 78, 216, 255), width=2)
    
    # Texte "S" blanc au centre
    try:
        font = ImageFont.load_default()
    except:
        font = None
    
    draw.text((size//2, size//2), "S", fill=(255, 255, 255, 255), font=font, anchor="mm")
    
    # Sauvegarder
    img.save('tray-icon.png')
    print("✅ Icône créée : tray-icon.png")
    
except ImportError:
    print("⚠️ PIL non disponible, création d'une icône de base...")
    # Créer un fichier PNG minimal (1x1 transparent)
    with open('tray-icon.png', 'wb') as f:
        # PNG minimal 1x1 transparent
        f.write(bytes([
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
            0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
            0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
            0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
            0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82
        ]))
    print("✅ Icône PNG minimal créée")
