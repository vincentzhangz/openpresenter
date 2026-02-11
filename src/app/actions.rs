pub enum Action {
    NextSlide,
    PreviousSlide,
    GoToSlide(usize),

    BlackScreen,
    LogoScreen,
    ClearScreen,

    SearchLibrary,
    ImportSong,

    NewPresentation,
    OpenPresentation,
    SavePresentation,
}
