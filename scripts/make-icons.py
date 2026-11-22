"""앱 아이콘 원본(1024px)을 그려요. 그다음 `tauri icon`이 모든 크기를 만들어요.

    npm run icons

트레이 아이콘(src-tauri/src/tray_icon.rs)과 같은 모양: 키 컬러 사각형 + 책꽂이.
"""
from pathlib import Path
from PIL import Image, ImageDraw

ACCENT = (0x8B, 0x7C, 0xF6, 255)
WHITE = (255, 255, 255, 255)
S = 1024
K = 4  # 슈퍼샘플링


def main():
    big = S * K
    img = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    u = big / 32  # 트레이 아이콘의 32px 격자를 그대로 키워요
    d.rounded_rectangle([2 * u, 2 * u, 30 * u, 30 * u], radius=7 * u, fill=ACCENT)
    for x0, y0, x1 in [(8, 9, 11), (13, 12, 16), (18, 8, 21)]:
        d.rounded_rectangle([x0 * u, y0 * u, x1 * u, 24 * u], radius=1 * u, fill=WHITE)
    d.rounded_rectangle([7 * u, 25 * u, 25 * u, 26.5 * u], radius=0.5 * u, fill=WHITE)
    img = img.resize((S, S), Image.LANCZOS)
    out = Path(__file__).with_name("icon-source.png")
    img.save(out)
    print(out)


if __name__ == "__main__":
    main()
