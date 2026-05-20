#!/usr/bin/env python3
import sys
import re
import requests
import subprocess
import os
import urllib.parse

def download_anibel(url):
    # 1. Выцягваем ID відэа
    match = re.search(r'video\.anibel\.net/([a-f0-9\-]+)', url)
    if not match:
        print("Памылка: Не атрымалася знайсці ID відэа ў спасылцы.")
        return
    video_id = match.group(1)
    
    # 2. Атрымліваем метаданыя праз API
    api_url = "https://api.anibel.stream/video"
    headers = {
        "Referer": "https://video.anibel.net/",
        "Origin": "https://video.anibel.net",
        "User-Agent": "Mozilla/5.0 (X11; Linux x86_64; rv:131.0) Gecko/20100101 Firefox/131.0"
    }
    data = {"videoId": video_id}
    
    print(f"[*] Атрыманне метаданых для ID: {video_id}...")
    try:
        resp = requests.post(api_url, headers=headers, data=data)
        resp.raise_for_status()
        meta = resp.json()
    except Exception as e:
        print(f"Памылка пры запыце да API: {e}")
        return
    
    title = meta.get("title", f"video_{video_id}.mkv")
    # Ачыстка імя файла ад забароненых сімвалаў
    title = re.sub(r'[\\/*?:"<>|]', "_", title)
    if not title.endswith('.mkv'):
        title += '.mkv'
        
    m3u8_url = f"{meta['host']}{meta['hls']}"
    subtitles = meta.get("subtitles", [])
    
    print(f"[*] Назва: {title}")
    
    # 3. Спампоўка відэа праз yt-dlp
    temp_video = f"temp_{video_id}.mp4"
    print(f"[*] Спампоўка відэа...")
    yt_cmd = [
        "yt-dlp",
        "--add-header", "Referer:https://video.anibel.net/",
        "--no-warning",
        "-o", temp_video,
        m3u8_url
    ]
    try:
        subprocess.run(yt_cmd, check=True)
    except subprocess.CalledProcessError:
        print("Памылка пры спампоўцы відэа праз yt-dlp.")
        return
    
    # 4. Спампоўка субтытраў
    sub_files = []
    for i, sub in enumerate(subtitles):
        sub_url = sub['path']
        # Дэкадуем URL для прыгожай назвы ў метаданых
        sub_filename = urllib.parse.unquote(os.path.basename(sub_url))
        sub_ext = sub_filename.split('.')[-1]
        
        local_sub_path = f"sub_{i}_{video_id}.{sub_ext}"
        print(f"[*] Спампоўка субтытраў: {sub_filename}...")
        
        try:
            sub_resp = requests.get(sub_url, headers=headers)
            sub_resp.raise_for_status()
            with open(local_sub_path, 'wb') as f:
                f.write(sub_resp.content)
            sub_files.append((local_sub_path, sub_filename.replace(f'.{sub_ext}', '')))
        except Exception as e:
            print(f"Папярэджанне: Не атрымалася спампаваць субтытры {sub_url}: {e}")
    
    # 5. Муксіраванне ў MKV праз ffmpeg
    print("[*] Зборка фінальнага файла...")
    ff_cmd = ["ffmpeg", "-y", "-i", temp_video]
    
    # Дадаем усе файлы субтытраў як уваходы
    for sub_path, _ in sub_files:
        ff_cmd.extend(["-i", sub_path])
    
    # Мапінг патокаў
    ff_cmd.extend(["-map", "0"]) # Відэа і аўдыё з асноўнага файла
    for i in range(len(sub_files)):
        ff_cmd.extend(["-map", str(i+1)]) # Кожны файл субтытраў
        
    ff_cmd.extend(["-c", "copy"]) # Капіруем без перакадавання
    
    # Дадаем назвы дарожак субтытраў
    for i, (_, sub_name) in enumerate(sub_files):
        ff_cmd.extend([f"-metadata:s:s:{i}", f"title={sub_name}"])
    
    ff_cmd.append(title)
    
    try:
        subprocess.run(ff_cmd, check=True, capture_output=True)
    except subprocess.CalledProcessError as e:
        print(f"Памылка ffmpeg: {e.stderr.decode()}")
        return
    
    # 6. Ачыстка часовых файлаў
    if os.path.exists(temp_video):
        os.remove(temp_video)
    for sub_path, _ in sub_files:
        if os.path.exists(sub_path):
            os.remove(sub_path)
            
    print(f"[+] Гатова! Файл захаваны як: {title}")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Выкарыстанне: ./anibel-dl.py <спасылка_на_відэа>")
        sys.exit(1)
    for link in sys.argv[1:]:
        download_anibel(link)
