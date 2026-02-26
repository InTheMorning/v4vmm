package main

import (
	"fmt"
	"io"
	"os"
	"path/filepath"

	id3v2 "github.com/bogem/id3v2/v2"
)

func readBasicID3(path string) (*ID3Info, error) {
	tag, err := id3v2.Open(path, id3v2.Options{Parse: true})
	if err != nil {
		return nil, fmt.Errorf("open id3 tag failed: %w", err)
	}
	defer tag.Close()

	info := &ID3Info{
		Title:  tag.Title(),
		Artist: tag.Artist(),
		Album:  tag.Album(),
		Other:  make(map[string][]string),
	}

	// AllFrames gives you every parsed frame in the tag.
	// Key = frame ID (e.g. "TIT2", "TPE1", "APIC"), value = list of frames.
	frames := tag.AllFrames()

	for frameID, list := range frames {
		// Skip the “basic” ones we already exposed separately
		if frameID == "TIT2" || frameID == "TPE1" || frameID == "TALB" {
			continue
		}

		// Convert each frame to a string (safe generic fallback)
		out := make([]string, 0, len(list))
		for _, fr := range list {
			out = append(out, fmt.Sprint(fr))
		}

		info.Other[frameID] = out
	}

	return info, nil
}

// A simple return type for "basic fields + everything else"
type ID3Info struct {
	Title  string
	Artist string
	Album  string
	Other  map[string][]string // frameID -> list of stringified frames
}

func writeBasicID3(path, title, artist, album string) error {
	// First try normal open+save (works when a tag exists or parsing succeeds)
	tag, err := id3v2.Open(path, id3v2.Options{Parse: true})
	if err == nil {
		defer tag.Close()

		tag.SetTitle(title)
		tag.SetArtist(artist)
		tag.SetAlbum(album)

		if err := tag.Save(); err != nil {
			return fmt.Errorf("save existing tag failed: %w", err)
		}
		return nil
	}

	// If parsing/open fails (often means "no ID3 header"), prepend a fresh tag
	newTag := id3v2.NewEmptyTag()
	newTag.SetDefaultEncoding(id3v2.EncodingUTF8)
	newTag.SetVersion(4) // ID3v2.4
	newTag.SetTitle(title)
	newTag.SetArtist(artist)
	newTag.SetAlbum(album)

	orig, err := os.Open(path)
	if err != nil {
		return fmt.Errorf("open mp3 failed: %w", err)
	}
	defer orig.Close()

	dir := filepath.Dir(path)
	tmp, err := os.CreateTemp(dir, ".tagged-*.mp3")
	if err != nil {
		return fmt.Errorf("create temp failed: %w", err)
	}
	tmpPath := tmp.Name()

	// If we error mid-way, clean up the temp file.
	defer func() {
		tmp.Close()
		_ = os.Remove(tmpPath)
	}()

	// 1) Write the new ID3 tag
	if _, err := newTag.WriteTo(tmp); err != nil {
		return fmt.Errorf("write new tag failed: %w", err)
	}

	// 2) Copy the original mp3 bytes after it
	if _, err := io.Copy(tmp, orig); err != nil {
		return fmt.Errorf("copy mp3 data failed: %w", err)
	}

	if err := tmp.Close(); err != nil {
		return fmt.Errorf("close temp failed: %w", err)
	}

	// Replace original file atomically-ish
	if err := os.Rename(tmpPath, path); err != nil {
		return fmt.Errorf("replace original failed: %w", err)
	}

	return nil
}

func main() {
	info, err := readBasicID3("test.mp3")
	if err != nil {
		fmt.Println("Read error:", err)
		return
	}
	fmt.Println("TITLE :", info.Title)
	fmt.Println("ARTIST:", info.Artist)
	fmt.Println("ALBUM :", info.Album)
	fmt.Println("OTHER frames:", len(info.Other))

	/*
			path := "test.mp3"
			if _, err := os.Stat(path); err != nil {
				fmt.Println("Put a copy of an MP3 in this folder named test.mp3")
				return
			}

			if err := writeBasicID3(path, "Test Title from Go", "Test Artist", "Test Album"); err != nil {
				fmt.Println("Error:", err)
				return
			}

		fmt.Println("✅ Wrote ID3 tags to", path)
	*/
}
