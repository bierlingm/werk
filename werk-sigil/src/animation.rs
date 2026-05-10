use std::path::PathBuf;

use crate::ctx::Ctx;
use crate::engine::Engine;
use crate::error::SigilError;
use crate::logic::Logic;
use crate::scope::Scope;

#[derive(Debug, Clone)]
pub enum AnimationAxis {
    SeedSweep { start: u64, end: u64, step: u64 },
    ParamSweep {
        stage: StageRef,
        param: String,
        from: f64,
        to: f64,
        frames: usize,
    },
    EpochRange { from: String, to: String },
}

#[derive(Debug, Clone)]
pub enum AnimationOutput {
    FrameSequence { dir: PathBuf },
    AnimatedSvg,
}

#[derive(Debug, Clone)]
pub struct AnimatedSigil {
    pub frames: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum StageRef {
    Selector,
    Featurizer,
    Encoder,
    Layouter,
    Stylist,
    Renderer,
}

pub fn render_animation(
    scope: Scope,
    logic: Logic,
    axis: AnimationAxis,
    output: AnimationOutput,
    ctx: &mut Ctx<'_>,
) -> Result<AnimatedSigil, SigilError> {
    let AnimationOutput::FrameSequence { dir } = output else {
        return Err(SigilError::unsupported("AnimatedSvg"));
    };

    let frames = match axis {
        AnimationAxis::SeedSweep { start, end, step } => {
            if step == 0 {
                return Err(SigilError::construction("seed sweep step must be > 0", 1, 1));
            }
            let mut paths = Vec::new();
            for seed in (start..end).step_by(step as usize) {
                let sigil = Engine::render_with_seed(scope.clone(), logic.clone(), ctx, Some(seed))?;
                let path = dir.join(format!("seed-{seed}.svg"));
                paths.push((path, sigil.svg.0));
            }
            write_frames(&dir, paths)?
        }
        AnimationAxis::ParamSweep {
            stage,
            param,
            from,
            to,
            frames,
        } => {
            validate_param(&logic, stage.clone(), &param)?;
            let steps = frames.max(1);
            let mut paths = Vec::new();
            for idx in 0..steps {
                let t = if steps == 1 {
                    0.0
                } else {
                    idx as f64 / (steps - 1) as f64
                };
                let value = from + (to - from) * t;
                let mut sweep_logic = logic.clone();
                set_param_value(&mut sweep_logic, stage.clone(), &param, value)?;
                let sigil = Engine::render(scope.clone(), sweep_logic, ctx)?;
                let path = dir.join(format!("frame-{idx:02}-{value:.1}.svg"));
                paths.push((path, sigil.svg.0));
            }
            write_frames(&dir, paths)?
        }
        AnimationAxis::EpochRange { .. } => return Err(SigilError::unsupported("EpochRange")),
    };

    Ok(AnimatedSigil { frames })
}

fn write_frames(
    dir: &PathBuf,
    frames: Vec<(PathBuf, Vec<u8>)>,
) -> Result<Vec<PathBuf>, SigilError> {
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| SigilError::io(format!("failed to create {}: {e}", parent.display())))?;
    }
    std::fs::create_dir_all(dir)
        .map_err(|e| SigilError::io(format!("failed to create {}: {e}", dir.display())))?;
    let mut paths = Vec::new();
    for (path, bytes) in frames {
        std::fs::write(&path, &bytes)
            .map_err(|e| SigilError::io(format!("failed to write {}: {e}", path.display())))?;
        paths.push(path);
    }
    Ok(paths)
}

fn validate_param(logic: &Logic, stage: StageRef, param: &str) -> Result<(), SigilError> {
    let (category, stage_name) = stage_target(logic, &stage)?;
    let Some(table) = logic.params.for_stage(category, stage_name) else {
        return Err(SigilError::construction(
            format!("unknown param {param}"),
            1,
            1,
        ));
    };
    let Some(table) = table.as_table() else {
        return Err(SigilError::construction(
            format!("unknown param {param}"),
            1,
            1,
        ));
    };
    match table.get(param) {
        Some(toml::Value::Float(_)) | Some(toml::Value::Integer(_)) => Ok(()),
        Some(_) => Err(SigilError::construction(
            format!("param {param} is not numeric"),
            1,
            1,
        )),
        None => Err(SigilError::construction(
            format!("unknown param {param}"),
            1,
            1,
        )),
    }
}

fn set_param_value(
    logic: &mut Logic,
    stage: StageRef,
    param: &str,
    value: f64,
) -> Result<(), SigilError> {
    let (category, stage_name) = stage_target_owned(logic, &stage)?;
    logic
        .params
        .set_param_number(&category, &stage_name, param, value);
    Ok(())
}

fn stage_target<'a>(logic: &'a Logic, stage: &StageRef) -> Result<(&'a str, &'a str), SigilError> {
    match stage {
        StageRef::Selector => Ok(("selector", logic.pipeline.selector.as_str())),
        StageRef::Featurizer => Ok(("featurizer", logic.pipeline.featurizer.as_str())),
        StageRef::Encoder => Ok(("encoder", logic.pipeline.encoder.as_str())),
        StageRef::Layouter => Ok(("layouter", logic.pipeline.layouter.as_str())),
        StageRef::Stylist => Ok(("stylist", logic.pipeline.stylist.as_str())),
        StageRef::Renderer => Ok(("renderer", logic.pipeline.renderer.as_str())),
    }
}

fn stage_target_owned(logic: &Logic, stage: &StageRef) -> Result<(String, String), SigilError> {
    let (category, stage_name) = stage_target(logic, stage)?;
    Ok((category.to_string(), stage_name.to_string()))
}
