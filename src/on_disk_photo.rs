use anyhow::{Result, bail};
use chrono::NaiveDateTime;
use image_hasher::HasherConfig;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::{fs, path::PathBuf};
use tokio::process::Command;
use tracing::{debug, error};

#[derive(Debug)]
pub struct PhotoMetadata {
    title: String,
    artist: Option<String>,
    copyright: Option<String>,
    date_taken: Option<NaiveDateTime>,
}

impl PhotoMetadata {
    pub fn title(&self) -> &String {
        &self.title
    }

    pub fn artist(&self) -> Option<&String> {
        self.artist.as_ref()
    }

    pub fn copyright(&self) -> Option<&String> {
        self.copyright.as_ref()
    }

    pub fn date_taken(&self) -> Option<&NaiveDateTime> {
        self.date_taken.as_ref()
    }
}

#[derive(Debug)]
pub struct OnDiskPhoto {
    hash: String,
    metadata: PhotoMetadata,
    fullsize: PathBuf,
    websize: PathBuf,
    thumbnail: PathBuf,
}

impl OnDiskPhoto {
    pub async fn import(orig: &Path, photos_dir: &Path, magick_exec: &Path) -> Result<Self> {
        let orig_name = orig.file_stem().unwrap().to_string_lossy().to_string();
        let orig_ext = orig.extension().unwrap().to_string_lossy();

        debug!("\n\ncheck 1\n\n");

        // TODO: the hash and parse_exif functions both open the photo file and can probably be optimized to do it once
        let hash = hash_photo(&orig)?;

        debug!("\n\ncheck 2\n\n");

        // Create a tmp working dir to work in.
        // If there are any errors, the tmp dir will be removed so we aren't left in a partial state.
        // Once we've done all failable operations, we move the tmpdir to the output location

        let tmp = tempfile::tempdir()?;
        let tmp_dir = tmp.path();
        let working_dir = tmp_dir.join(&hash);
        if working_dir.exists() {
            bail!(
                "Output dir for input {} already exists (photo has already been imported)",
                orig.display()
            );
        }
        fs::create_dir_all(&working_dir)?;

        debug!("\n\ncheck 3\n\n");

        let title = orig_name;
        let (artist, copyright, date_taken) = parse_exif(orig)?;
        let metadata = PhotoMetadata {
            title,
            artist,
            copyright,
            date_taken,
        };
        debug!("metadata: {metadata:?}");

        debug!("\n\ncheck 4\n\n");

        let websize_name = format!("{hash}-websize.{orig_ext}");
        let tmp_websize = working_dir.join(&websize_name);
        make_websize(magick_exec, &orig, &tmp_websize).await?;
        debug!("Generated websize image {}", tmp_websize.display());

        debug!("\n\ncheck 5\n\n");

        let thumbnail_name = format!("{hash}-thumbnail.{orig_ext}");
        let tmp_thumbnail = working_dir.join(&thumbnail_name);
        make_thumbnail(magick_exec, &orig, &tmp_thumbnail).await?;
        debug!("Generated thumbnail image {}", tmp_thumbnail.display());

        debug!("\n\ncheck 6\n\n");

        let fullsize_name = format!("{hash}-fullsize.{orig_ext}");
        let fullsize_tmp = working_dir.join(&fullsize_name);
        // Note: fs::rename fails when /tmp and data dir are on different filesystems so we use copy+remove instead
        // This was triggered when running pictureframe under systemd
        // https://stackoverflow.com/a/24210631
        // fs::rename(orig, &fullsize_tmp)?;
        fs::copy(&orig, &fullsize_tmp)?;
        fs::remove_file(&orig)?;

        debug!("\n\ncheck 7\n\n");

        // TODO: can we add a clippy lint to block the usage of fs::rename()?

        // Move the entire working directory to the final output location
        // TODO: this fails if we've previously processed the photo. We need to use the hash to check the database before we get this far. Should be done before generating thumbs too
        let outdir = photos_dir.join(&hash);
        let mut opts = fs_extra::dir::CopyOptions::new();
        opts.copy_inside = true;
        fs_extra::dir::copy(&working_dir, &outdir, &opts)?;
        // working_dir will get deleted on drop

        debug!("\n\ncheck 8\n\n");

        // The paths now point to the new location after the rename
        let fullsize = outdir.join(&fullsize_name);
        let websize = outdir.join(&websize_name);
        let thumbnail = outdir.join(&thumbnail_name);

        debug!("\n\ncheck 9\n\n");

        Ok(Self {
            hash,
            metadata,
            fullsize,
            websize,
            thumbnail,
        })
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn metadata(&self) -> &PhotoMetadata {
        &self.metadata
    }

    pub fn fullsize_path(&self) -> &Path {
        &self.fullsize
    }

    pub fn websize_path(&self) -> &Path {
        &self.websize
    }

    pub fn thumbnail_path(&self) -> &Path {
        &self.thumbnail
    }
}

fn hash_photo(path: &Path) -> Result<String> {
    let img = image::open(path)?;
    let hasher = HasherConfig::new().to_hasher();
    let hash = hasher.hash_image(&img);
    let hash_str = hash.to_base64();
    Ok(hash_str)
}

async fn magick(magick_exec: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new(magick_exec).args(args).output().await?;
    let status = output.status;

    if !status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Command failed. Stderr:\n{stderr}");
        bail!(
            "Command {} with args {args:?} exited with {status}",
            magick_exec.display()
        );
    }
    Ok(())
}

async fn make_websize(magick_exec: &Path, input: &Path, outpath: &Path) -> Result<()> {
    // TODO: tune args like quality
    let input = input.as_os_str().to_str().unwrap();
    let outpath = outpath.as_os_str().to_str().unwrap();
    magick(
        magick_exec,
        &[
            input,
            "-strip",
            "-interlace",
            "Plane",
            "-sampling-factor",
            "4:2:0",
            "-quality",
            "82",
            outpath,
        ],
    )
    .await
}

async fn make_thumbnail(magick_exec: &Path, input: &Path, outpath: &Path) -> Result<()> {
    // TODO: tune args like quality
    let input = input.as_os_str().to_str().unwrap();
    let outpath = outpath.as_os_str().to_str().unwrap();
    magick(
        magick_exec,
        &[
            input,
            "-resize",
            "320x320>",
            "-filter",
            "Triangle",
            "-strip",
            "-colorspace",
            "sRGB",
            "-interlace",
            "Plane",
            "-sampling-factor",
            "4:2:0",
            "-quality",
            "60",
            outpath,
        ],
    )
    .await
}

fn parse_exif(path: &Path) -> Result<(Option<String>, Option<String>, Option<NaiveDateTime>)> {
    debug!("Parsing exif data");
    let f = File::open(&path)?;
    let mut reader = BufReader::new(&f);
    let exifreader = exif::Reader::new();
    let Ok(exif) = exifreader.read_from_container(&mut reader) else {
        return Ok((None, None, None));
    };

    let artist = exif
        .get_field(exif::Tag::Artist, exif::In::PRIMARY)
        .map(|field| field.value.display_as(exif::Tag::Artist).to_string());
    let copyright = exif
        .get_field(exif::Tag::Copyright, exif::In::PRIMARY)
        .map(|field| field.value.display_as(exif::Tag::Copyright).to_string());
    let date_taken = exif
        .get_field(exif::Tag::DateTime, exif::In::PRIMARY)
        .map(|field| field.value.display_as(exif::Tag::DateTime).to_string());

    let date_taken = match date_taken {
        Some(raw) => {
            let parsed = NaiveDateTime::parse_from_str(&raw, "%Y:%m:%d %H:%M:%S")?;
            Some(parsed)
        }
        None => None,
    };

    Ok((artist, copyright, date_taken))
}
