use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;

use std::path::{Path, PathBuf};

pub const TEMP_FOLDER_NAME: &'static str = "__v_customizer_temp__";
const TEMP_MODELS_NAME: &'static str = "__TEMP_MODELS";
const CLASSES: [Class; 9] = [
    Class::Scout,
    Class::Soldier,
    Class::Pyro,
    Class::Demo,
    Class::Heavy,
    Class::Engineer,
    Class::Medic,
    Class::Sniper,
    Class::Spy,
];

#[derive(Deserialize, Serialize, Clone, Copy, Default)]
pub struct Origin {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub z_rot: f32,
}

impl Origin {
    pub fn reset(&mut self) {
        self.x = 0.0;
        self.y = 0.0;
        self.z = 0.0;
        self.z_rot = 0.0;
    }

    fn is_modified(&self) -> bool {
        if self.x == 0.0 && self.y == 0.0 && self.z == 0.0 && self.z_rot == 0.0 {
            false
        } else {
            true
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq)]
pub enum Class {
    Scout,
    Soldier,
    Pyro,
    Demo,
    Heavy,
    Engineer,
    Medic,
    Sniper,
    Spy,
}

impl Default for Class {
    fn default() -> Self {
        Class::Scout
    }
}

impl ToString for Class {
    // assumes the folder names won't change
    fn to_string(&self) -> String {
        match self {
            Class::Scout => "Scout".to_owned(),
            Class::Soldier => "Soldier".to_owned(),
            Class::Pyro => "Pyro".to_owned(),
            Class::Demo => "Demo".to_owned(),
            Class::Heavy => "Heavy".to_owned(),
            Class::Engineer => "Engineer".to_owned(),
            Class::Medic => "Medic".to_owned(),
            Class::Sniper => "Sniper".to_owned(),
            Class::Spy => "Spy".to_owned(),
        }
    }
}

impl AsRef<Path> for Class {
    fn as_ref(&self) -> &Path {
        match self {
            Class::Scout => Path::new("Scout"),
            Class::Soldier => Path::new("Soldier"),
            Class::Pyro => Path::new("Pyro"),
            Class::Demo => Path::new("Demo"),
            Class::Heavy => Path::new("Heavy"),
            Class::Engineer => Path::new("Engineer"),
            Class::Medic => Path::new("Medic"),
            Class::Sniper => Path::new("Sniper"),
            Class::Spy => Path::new("Spy"),
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct Animation {
    // animation folder name
    pub name: String,
    // $origin positions
    pub origin: Origin,
}

#[derive(Deserialize, Serialize, Default)]
pub struct ClassAnimations {
    pub class: Class,
    pub animations: Vec<Animation>,
}

impl ClassAnimations {
    pub fn get_selected_animations(&self) -> Vec<&Animation> {
        self.animations
            .iter()
            .filter(|animation| animation.origin.is_modified())
            .collect::<Vec<&Animation>>()
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct Sca {
    // there should be 9 folders for the 9 classes
    pub folders: Vec<ClassAnimations>,
}

impl Sca {
    // creates Sca from the SCA folder
    pub fn new() -> Result<Self> {
        Self::check_folders()?;
        let sca_dir = Self::sca_folder()?;
        let mut sca = Sca {
            folders: Vec::new(),
        };
        for class in CLASSES {
            let class_path = sca_dir.join(class);
            let class_folder = std::fs::read_dir(class_path)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir());
            sca.folders.push(ClassAnimations {
                class,
                animations: class_folder
                    .map(|folder| Animation {
                        name: folder
                            .path()
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string(),
                        origin: Origin::default(),
                    })
                    .collect::<Vec<Animation>>(),
            });
        }
        Ok(sca)
    }

    // something is selected if its origin is isn't all 0.0
    pub fn get_selected_classes(&self) -> Vec<&ClassAnimations> {
        self.folders
            .iter()
            .filter(|folder| {
                folder
                    .animations
                    .iter()
                    .any(|animation| animation.origin.is_modified())
            })
            .collect::<Vec<&ClassAnimations>>()
    }

    pub fn get_selected_class_qcs(&self) -> Result<Vec<PathBuf>> {
        let mut result = Vec::new();
        for class in self.get_selected_classes() {
            let class_folder = Self::exe_folder()?
                .join(TEMP_FOLDER_NAME)
                .join(&class.class);
            let class_qc_dir = std::fs::read_dir(class_folder)?
                .filter_map(|res| res.ok())
                .filter(|res| res.path().extension() == Some(std::ffi::OsStr::new("qc")))
                .next()
                .ok_or(anyhow!("Can't find class qc"))?
                .path();
            result.push(class_qc_dir);
        }
        Ok(result)
    }

    // pub fn compile_class(&self) -> Result<()> {
    //     for qc_file in self.get_temp_folder_qcs(false)? {
    //         Self::compile(qc_file)?;
    //     }
    //     for class_qc_dir in self.get_selected_class_qcs()? {
    //         Self::compile(class_qc_dir)?;
    //     }
    //     Ok(())
    // }

    // compiles using studiomdl.exe
    pub fn compile<P: AsRef<Path>>(qc_file: P) -> Result<()> {
        let output = std::process::Command::new(Self::studiomdl_exe()?)
            .arg("-game")
            .arg(Self::tf_folder()?)
            .args(["-nop4", "-verbose"])
            .arg(qc_file.as_ref())
            .creation_flags(0x00000008)
            .output()?;
        println!("{}", std::str::from_utf8(&output.stdout).unwrap());
        if !output.status.success() {
            bail!("studiomdl.exe didn't exit with exit code 0");
        }
        Ok(())
    }

    // converts to vpk and moves to custom
    pub fn convert_to_vpk() -> Result<()> {
        let vpk_exe = Self::studiomdl_exe()?.with_file_name("vpk.exe");
        let temp_model_folder = Self::tf_folder()?
            .join("models")
            .join("__TEMP")
            .join("0_ViewmodelCustomized");
        let output = std::process::Command::new(vpk_exe)
            .arg(&temp_model_folder)
            .creation_flags(0x00000008)
            .output()?;
        println!("{}", std::str::from_utf8(&output.stdout).unwrap());
        if !output.status.success() {
            bail!("vpk.exe didn't exit with exit code 0");
        }
        std::fs::rename(
            temp_model_folder.with_file_name("0_ViewmodelCustomized.vpk"),
            Self::tf_folder()?
                .join("custom")
                .join("0_ViewmodelCustomized.vpk"),
        )?;
        Ok(())
    }

    pub fn create_temp_models_folder() -> Result<()> {
        let models_folder = Self::tf_folder()?.join("models");
        if !models_folder.exists() {
            std::fs::create_dir_all(&models_folder)?;
        }
        std::fs::rename(
            &models_folder,
            models_folder.with_file_name(TEMP_MODELS_NAME),
        )?;
        Ok(())
    }

    pub fn delete_temp_models_folder() -> Result<()> {
        let models_folder = Self::tf_folder()?.join("models");
        let temp_models_folder = models_folder.clone().with_file_name(TEMP_MODELS_NAME);
        if temp_models_folder.exists() {
            std::fs::remove_dir_all(&models_folder)?;
            std::fs::rename(&temp_models_folder, &models_folder)?;
        }
        Ok(())
    }

    // gets the animation .qc files in the temp folder that were copied from the SCA folder
    pub fn get_temp_folder_qcs(&self, selected_only: bool) -> Result<Vec<PathBuf>> {
        let mut result = Vec::new();
        for class in self.get_selected_classes() {
            let animations = if selected_only {
                class.get_selected_animations()
            } else {
                class
                    .animations
                    .iter()
                    .map(|animation| animation)
                    .collect::<Vec<&Animation>>()
            };
            for animation in animations {
                let anim_folder_dir = Self::exe_folder()?
                    .join(TEMP_FOLDER_NAME)
                    .join(&class.class)
                    .join(&animation.name);
                // assumes there will on be one qc file in each animation folder
                let qc_file_dir = std::fs::read_dir(anim_folder_dir)?
                    .filter_map(|res| res.ok())
                    .filter(|res| res.path().extension() == Some(std::ffi::OsStr::new("qc")))
                    .next()
                    .ok_or(anyhow!("Can't find class qc"))?
                    .path();
                result.push(qc_file_dir);
            }
        }
        Ok(result)
    }

    // adds $origin to top of each selected weapon's qc file
    pub fn append_origins(&self) -> Result<()> {
        for class in self.get_selected_classes() {
            for animation in class.get_selected_animations() {
                let anim_folder_dir = Self::exe_folder()?
                    .join(TEMP_FOLDER_NAME)
                    .join(&class.class)
                    .join(&animation.name);
                let qc_file_dir = std::fs::read_dir(anim_folder_dir)?
                    .filter_map(|res| res.ok())
                    .filter(|res| res.path().extension() == Some(std::ffi::OsStr::new("qc")))
                    .next()
                    .ok_or(anyhow!("Can't find class qc"))?
                    .path();
                let qc_file_content = std::fs::read_to_string(&qc_file_dir)?;
                std::fs::write(
                    &qc_file_dir,
                    format!(
                        "$origin {} {} {} {}\n{}",
                        animation.origin.x,
                        animation.origin.y,
                        animation.origin.z,
                        animation.origin.z_rot,
                        qc_file_content,
                    ),
                )?;
            }
        }
        Ok(())
    }

    // copys SCA folder and its selected classes
    pub fn copy_sca(&self) -> Result<()> {
        let sca_dir = Self::sca_folder()?;
        let temp_folder = Self::exe_folder()?.join(TEMP_FOLDER_NAME);
        std::fs::create_dir_all(&temp_folder)?;
        let selected_classes = self.get_selected_classes();
        let classes = selected_classes
            .iter()
            .map(|classes| classes.class.to_string())
            .collect::<Vec<String>>();
        let class_dirs = classes
            .iter()
            .map(|classes| Path::new(&sca_dir).join(classes))
            .collect::<Vec<PathBuf>>();
        for class in &classes {
            std::fs::create_dir_all(&temp_folder.join(class))?
        }
        for (class_dir, class) in class_dirs.iter().zip(classes.iter()) {
            for res in std::fs::read_dir(class_dir)? {
                let entry = res?.path();
                match entry.is_file() {
                    true => {
                        std::fs::copy(
                            &entry,
                            // assumes there won't be an empty file name
                            &temp_folder.join(&class).join(&entry.file_name().unwrap()),
                        )?;
                    }
                    // assumes there are only two nested folders
                    false => {
                        let animation_folder =
                            &temp_folder.join(&class).join(&entry.file_name().unwrap());
                        std::fs::create_dir_all(animation_folder)?;
                        for res in std::fs::read_dir(&entry)? {
                            let entry = res?.path();
                            if entry.is_dir() {
                                bail!("Folder found in an animation folder {}", entry.display());
                            }
                            std::fs::copy(
                                &entry,
                                &animation_folder.join(&entry.file_name().unwrap()),
                            )?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn reset_origin<T: AsRef<str>>(&mut self, class: &Class, name: T) {
        for folder in self.folders.iter_mut() {
            if folder.class == *class {
                for animation in folder.animations.iter_mut() {
                    if animation.name == name.as_ref() {
                        animation.origin.reset();
                        break;
                    }
                }
                break;
            }
        }
    }

    pub fn reset_all_origin(&mut self) {
        self.folders.iter_mut().for_each(|folder| {
            folder
                .animations
                .iter_mut()
                .for_each(|animation| animation.origin.reset())
        });
    }

    pub fn apply_to_all_origin(&mut self, origin: &Origin) {
        for folder in self.folders.iter_mut() {
            for animation in folder.animations.iter_mut() {
                animation.origin = origin.clone();
            }
        }
    }

    pub fn check_folders() -> Result<()> {
        let sca_dir = Self::sca_folder()?;
        if !sca_dir.is_dir() {
            bail!("Can't find SCA folder");
        } else {
            for class in CLASSES {
                if !sca_dir.join(&class).is_dir() {
                    bail!("Can't find {} folder", class.to_string());
                }
            }
        }
        Ok(())
    }

    pub fn exe_folder() -> Result<PathBuf> {
        let mut exe_folder = std::env::current_exe()?;
        exe_folder.pop();
        Ok(exe_folder)
    }

    pub fn studiomdl_exe() -> Result<PathBuf> {
        let mut exe_folder = Self::exe_folder()?;
        // assumes program folder is in custom folder
        (0..3).for_each(|_| {
            exe_folder.pop();
        });
        let smdl = exe_folder.join("bin").join("studiomdl.exe");
        if !smdl.exists() {
            bail!("Can't find studiomdl.exe, program's folder has to be in your custom folder!");
        }
        Ok(smdl)
    }

    pub fn tf_folder() -> Result<PathBuf> {
        let mut tf_folder = Self::exe_folder()?;
        tf_folder.pop();
        tf_folder.pop();
        if !tf_folder.exists() {
            bail!("Can't find tf folder, program's folder has to be in your custom folder!")
        }
        Ok(tf_folder)
    }

    fn sca_folder() -> Result<PathBuf> {
        Ok(Self::exe_folder()?.join("SCA"))
    }
}
