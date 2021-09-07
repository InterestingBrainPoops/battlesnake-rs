use super::*;

pub struct ConstantCarter {}

impl<T> BattlesnakeAI<T> for ConstantCarter {
    fn name(&self) -> String {
        "constant-carter".to_owned()
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            author: Some("coreyja".to_owned()),
            color: Some("#AA66CC".to_owned()),
            ..Default::default()
        }
    }

    fn make_move(
        &self,
        _state: Game,
    ) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        Ok(MoveOutput {
            r#move: format!("{}", Move::Right),
            shout: None,
        })
    }
}
