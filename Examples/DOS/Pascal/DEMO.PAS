{ ==========================================================================
  DEMO.PAS - the OWLOS UI demo, in Pascal.

      RUN DEMO

  The shape every text-mode toolkit's demo has had, because it is the right
  shape: a menu bar, a status line, a desktop, and small windows that each
  use one part of the kit.

    File      New: an editor window of its own. Open: the file panel,
              the folder read through DOS, a file shown in a viewer. Exit.
    Tools     Calculator  - a keypad of button rows; typing works too,
                            because the window carries the keys it wants
              Calendar    - a canvas drawn again for every month
              ASCII table - every glyph of the font; a click names one
              Puzzle      - the fifteen puzzle: click a tile, or arrows
    Window    Zoom, Next, Previous, Close, List, Cascade, Tile
    Help      About

  The same demo is in Examples\DOS\Asm and Examples\DOS\C, part for part.
  The toolkit - every frame, menu, button and the editor - is OWLOSRES,
  reached through INT 60h (lib\dos\pascal\OWLOSUI.PAS). What is here is
  the program: what to build, and what to do with each command.
  ========================================================================== }

program Demo;

uses Dos, OwlosUI;

const
  { The program's own command numbers; 0 means none. }
  CmNew = 1;        CmExit = 2;       CmOpen = 3;
  CmCalc = 10;      CmCalendar = 11;  CmAscii = 12;    CmPuzzle = 13;
  CmNext = 30;      CmZoom = 31;      CmClose = 32;    CmCascade = 33;
  CmTile = 34;      CmPrevious = 35;  CmList = 36;
  CmAbout = 40;     CmHelp = 41;      CmDismiss = 62;
  CmFileOpen = 63;  CmOpenCancel = 64;               { the Open dialog's buttons }
  { the calculator's keys }
  CmDigit = 100;    { 100..109: 0..9 }
  CmAdd = 110;      CmSub = 111;      CmMul = 112;     CmDiv = 113;
  CmEquals = 114;   CmClear = 115;    CmNegate = 116;
  { the calendar's }
  CmMonthBack = 160; CmMonthOn = 161; CmToday = 162;
  { the puzzle's }
  CmScramble = 180; CmUp = 181; CmDown = 182; CmLeft = 183; CmRight = 184;

  MinLong = -2147483647 - 1;
  MaxLong = 2147483647;

var
  Box, CalcWin, CalWin, AsciiWin, PuzWin: Word;
  Editors: Integer;
  Glyphs: TGlyphs;

function IntStr(N: LongInt): string;
var
  S: string;
begin
  Str(N, S);
  IntStr := S;
end;

{ ---- about, and the one message box ------------------------------------- }

procedure BoxClose;
begin
  if Box <> 0 then OwlClose(Box);
  Box := 0;
end;

procedure About;
begin
  BoxClose;
  OwlMessageBox('OWLOS UI Demo',
    'A Pascal program on DOS. Every window in it is drawn by the OWLOSUI core, ' +
    'a Rust program behind INT 60h - the same core that draws the C# demo on ' +
    'Windows and the JavaScript one in a browser.', 1);
  OwlButton(CmDismiss, OwlDefault, '~O~K');
  Box := OwlEnd;
end;

{ ---- File > New: an editor, a window like any other --------------------- }

procedure EditorNew;
var
  W: Word;
  N: Integer;
begin
  Inc(Editors);
  N := Editors and 7;
  W := OwlWindow(0, 2 + N, 1 + N, 56, 14, OwlBlue, 'UNTITLED' + IntStr(Editors) + '.TXT', CmClose);
  { Everything the editor has, on an Edit menu of its own while this window
    is in front: the core runs it all, nothing comes back here. }
  OwlEditor(OwlText(W, 'Type here. Shift and the arrows select, Ctrl+Z takes it back. ' +
                       'The Edit menu has Find, Replace and Word wrap.'), OfferAll, 0);
end;

{ ---- File > Open: the file panel, the folder read through DOS ----------- }

var
  OpenDialog, OpenPanel: Word;
  OpenDir: string;                      { the folder the panel shows }
  FileBuf: array[0..8191] of Byte;      { a file's first 8 KB, for the viewer }

procedure OpenShow;
begin
  if OpenDialog <> 0 then begin OwlActivate(OpenDialog); Exit; end;
  if OpenDir = '' then GetDir(0, OpenDir);
  OpenDialog := OwlWindow(0, -1, -1, 70, 20, OwlDialog or OwlModal, 'Open', CmOpenCancel);
  OpenPanel := OwlFiles(OpenDialog, 1, OpenDir);        { a row of air above it }
  OwlButtons(OpenDialog, 2);
  OwlButton(CmFileOpen, OwlDefault, '~O~pen');
  OwlButton(CmOpenCancel, OwlCancel, '~C~ancel');
  OwlEnd;
end;

procedure OpenClose;
begin
  if OpenDialog <> 0 then OwlClose(OpenDialog);
  OpenDialog := 0;
  OpenPanel := 0;
end;

{ Where the last backslash is; 0 if there is none. }
function LastSlash(const S: string): Integer;
var
  I: Integer;
begin
  I := Length(S);
  while (I > 0) and (S[I] <> '\') do Dec(I);
  LastSlash := I;
end;

{ A name made into a whole path: a drive's path as it is, anything else
  from the folder the panel shows. }
function Join(const Dir, Name: string): string;
begin
  if (Length(Name) >= 2) and (Name[2] = ':') then Join := Name
  else if (Length(Name) >= 1) and (Name[1] = '\') then Join := Copy(Dir, 1, 2) + Name
  else if Dir[Length(Dir)] = '\' then Join := Dir + Name
  else Join := Dir + '\' + Name;
end;

{ Everything before the last backslash - C:\A\B becomes C:\A, C:\A
  becomes C:\. }
function FolderOf(const Path: string): string;
var
  Cut: Integer;
begin
  Cut := LastSlash(Path);
  if Cut = 0 then FolderOf := Path
  else if Cut = 3 then FolderOf := Copy(Path, 1, 3)
  else FolderOf := Copy(Path, 1, Cut - 1);
end;

function HasMask(const S: string): Boolean;
begin
  HasMask := (Pos('*', S) > 0) or (Pos('?', S) > 0);
end;

{ The panel shows another folder - or says why it cannot. A path with a
  mask on the end, as the path line shows it, means its folder. }
procedure GoFolder(Path: string);
begin
  if HasMask(Path) then Path := FolderOf(Path);
  if OwlSetFiles(OpenPanel, Path) then OpenDir := Path
  else OwlFilesError(OpenPanel, 'Folder not found: ' + Path);
end;

{ A file into a viewer window of its own: its first 8 KB, read by DOS. }
function ViewFile(const Path, Name: string): Boolean;
var
  F: file;
  N: Word;
  Size: LongInt;
  Title: string;
  W, T: Word;
begin
  ViewFile := False;
  Assign(F, Path);
  {$I-} Reset(F, 1); {$I+}
  if IOResult <> 0 then Exit;
  BlockRead(F, FileBuf, SizeOf(FileBuf), N);
  Size := FileSize(F);
  Close(F);
  if Size > N then Title := Name + ' - the first ' + IntStr(N) + ' bytes'
  else Title := Name;
  W := OwlWindow(0, 2, 2, 76, 20, OwlBlue, Title, CmClose);
  { A viewer, with Find, Word wrap, the bytes in hex and the language on its
    Edit menu - and coloured as the language its name says. }
  T := OwlTextDos(W, FileBuf, N, 1);
  OwlEditor(T, OfferFind or OfferWrap or OfferHex or OfferSyntax, EditReadOnly);
  OwlSyntax(T, Name);
  ViewFile := True;
end;

{ A name entered in the panel, or its path typed: a folder is walked into,
  a file is opened and the dialog closes. }
procedure NameChosen(const Name: string);
var
  Full: string;
  F: file;
  Attr: Word;
begin
  if Name = '..' then begin GoFolder(FolderOf(OpenDir)); Exit; end;
  Full := Join(OpenDir, Name);
  Assign(F, Full);
  GetFAttr(F, Attr);
  if DosError <> 0 then begin OwlFilesError(OpenPanel, 'Not found: ' + Full); Exit; end;
  if Attr and Directory <> 0 then begin GoFolder(Full); Exit; end;
  if ViewFile(Full, Copy(Full, LastSlash(Full) + 1, 12)) then OpenClose
  else OwlFilesError(OpenPanel, 'Cannot read: ' + Full);
end;

{ What the panel reports is not a command: asked for after every event. }
procedure OpenPoll;
var
  Text: string;
  Kind: Byte;
begin
  if OpenDialog = 0 then Exit;
  Kind := OwlTakeFiles(OpenPanel, Text);
  if Kind = 1 then NameChosen(Text)
  else if Kind = 2 then
  begin
    Text := Join(OpenDir, Text);
    if HasMask(Text) then GoFolder(Text) else NameChosen(Text);
  end;
end;

{ The Open button: whatever is under the cursor. }
procedure OpenButton;
var
  Name: string;
begin
  if (OpenPanel <> 0) and OwlCurrentName(OpenPanel, Name) then NameChosen(Name);
end;

{ ---- the calculator: a pocket calculator's arithmetic on LongInts ------- }

var
  CalcDisplay: Word;
  Acc, Entry: LongInt;
  Op: Word;
  Typing, Error: Boolean;

{ The display is a canvas one line high, so the number can stand at its
  right-hand end - a static line would fold the spaces away. }
procedure CalcPrint(Value: LongInt);
var
  S: string;
  I, Pad: Integer;
begin
  if Error then S := 'Error' else S := IntStr(Value);
  Pad := 40 - Length(S);
  OwlBlit(CalcDisplay, 0, 0, 40, 1);
  for I := 0 to 39 do
    if I < Pad then OwlCell(Ord(' '), $1F) else OwlCell(Ord(S[I - Pad + 1]), $1F);
  OwlEnd;
end;

{ Acc := Acc (op) Entry; an overflow or a division by nought is an error.
  The check is done in floating point first, where nothing overflows. }
procedure CalcApply;
var
  R: LongInt;
  F: Real;
begin
  R := Acc;
  if Op = 0 then R := Entry             { nothing waiting: the number is the total }
  else if Typing then
  begin
    case Op of
      CmAdd: F := Acc + 1.0 * Entry;
      CmSub: F := Acc - 1.0 * Entry;
      CmMul: F := 1.0 * Acc * Entry;
      CmDiv: if Entry = 0 then F := 1e30 else F := 1.0 * Acc / Entry;
    end;
    if (F > MaxLong) or (F < MinLong) then Error := True
    else case Op of
      CmAdd: R := Acc + Entry;
      CmSub: R := Acc - Entry;
      CmMul: R := Acc * Entry;
      CmDiv: R := Acc div Entry;
    end;
  end;
  if not Error then Acc := R;
  Typing := False;
end;

procedure CalcKey(Cmd: Word);
var
  D: LongInt;
begin
  if CalcWin = 0 then Exit;
  if Cmd = CmClear then
  begin
    Acc := 0; Entry := 0; Op := 0;
    Typing := False; Error := False;
    CalcPrint(Acc);
    Exit;
  end;
  if Error then Exit;                   { after an error only C does anything }
  if Cmd < CmAdd then                   { a digit }
  begin
    D := Cmd - CmDigit;
    if not Typing then begin Entry := 0; Typing := True; end;
    if (Entry <= 99999999) and (Entry >= -99999999) then   { nine digits are enough }
      if Entry < 0 then Entry := Entry * 10 - D else Entry := Entry * 10 + D;
    CalcPrint(Entry);
  end
  else if Cmd = CmEquals then
  begin
    CalcApply;
    Op := 0;
    Entry := Acc;
    CalcPrint(Acc);
  end
  else if Cmd = CmNegate then
  begin
    if Typing then begin Entry := -Entry; CalcPrint(Entry); end
    else begin Acc := -Acc; Entry := Acc; CalcPrint(Acc); end;
  end
  else                                  { + - * /: what waited is done, this one waits }
  begin
    CalcApply;
    Op := Cmd;
    CalcPrint(Acc);
  end;
end;

procedure CalcShow;
var
  I: Integer;
begin
  if CalcWin <> 0 then begin OwlActivate(CalcWin); Exit; end;
  CalcWin := OwlWindow(0, -1, -1, 46, 13, OwlDialog, 'Calculator', CmClose);
  CalcDisplay := OwlCanvas(CalcWin, 2, 1, 40, 1);
  { The keypad: four placed rows, labels padded to three so the columns
    line up; a keypad is for the mouse and for typing, not for Tab. }
  OwlButtonRow(CalcWin, 2, 3, 5);
  OwlButton(CmDigit + 7, 0, ' 7 '); OwlButton(CmDigit + 8, 0, ' 8 '); OwlButton(CmDigit + 9, 0, ' 9 ');
  OwlButton(CmDiv, OwlAccent, ' / '); OwlButton(CmClear, OwlDanger, ' C ');
  OwlEnd;
  OwlButtonRow(CalcWin, 2, 5, 4);
  OwlButton(CmDigit + 4, 0, ' 4 '); OwlButton(CmDigit + 5, 0, ' 5 '); OwlButton(CmDigit + 6, 0, ' 6 ');
  OwlButton(CmMul, OwlAccent, ' * ');
  OwlEnd;
  OwlButtonRow(CalcWin, 2, 7, 4);
  OwlButton(CmDigit + 1, 0, ' 1 '); OwlButton(CmDigit + 2, 0, ' 2 '); OwlButton(CmDigit + 3, 0, ' 3 ');
  OwlButton(CmSub, OwlAccent, ' - ');
  OwlEnd;
  OwlButtonRow(CalcWin, 2, 9, 4);
  OwlButton(CmDigit + 0, 0, ' 0 '); OwlButton(CmNegate, OwlAccent, '+/-');
  OwlButton(CmEquals, OwlAccent or OwlDefault, ' = '); OwlButton(CmAdd, OwlAccent, ' + ');
  OwlEnd;
  { The keys the window carries: while it is in front, a digit or an
    operator typed is its command. No labels: the status line keeps its
    own words. Enter is the default button, =. }
  OwlWindowStatus(CalcWin, 17);
  for I := 0 to 9 do OwlKey('', CmDigit + I, KeyChar, Ord('0') + I, 0);
  OwlKey('', CmAdd, KeyChar, Ord('+'), 0);
  OwlKey('', CmSub, KeyChar, Ord('-'), 0);
  OwlKey('', CmMul, KeyChar, Ord('*'), 0);
  OwlKey('', CmDiv, KeyChar, Ord('/'), 0);
  OwlKey('', CmEquals, KeyChar, Ord('='), 0);
  OwlKey('', CmClear, KeyNamed, 1, 0);  { Esc }
  OwlKey('', CmClear, KeyChar, Ord('c'), 0);
  OwlEnd;
  CalcKey(CmClear);
end;

{ ---- the calendar -------------------------------------------------------- }

const
  MonthNames: array[1..12] of string[9] = (
    'January', 'February', 'March', 'April', 'May', 'June',
    'July', 'August', 'September', 'October', 'November', 'December');
  DayNames: string[21] = 'Su Mo Tu We Th Fr Sa ';

var
  CalTitle, CalCanvas: Word;
  CalYear, CalMonth, TodayYear, TodayMonth, TodayDay: Integer;

{ 0 Sunday .. 6 Saturday. Sakamoto's: January and February counted in the
  year before. }
function DayOfWeek(Y, M, D: Integer): Integer;
const
  T: array[1..12] of Integer = (0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4);
begin
  if M < 3 then Dec(Y);
  DayOfWeek := (Y + Y div 4 - Y div 100 + Y div 400 + T[M] + D) mod 7;
end;

function DaysInMonth(Y, M: Integer): Integer;
const
  Days: array[1..12] of Integer = (31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31);
var
  Leap: Boolean;
begin
  Leap := ((Y mod 4 = 0) and (Y mod 100 <> 0)) or (Y mod 400 = 0);
  if (M = 2) and Leap then DaysInMonth := 29 else DaysInMonth := Days[M];
end;

{ The month into the canvas: a row of day names, then up to six weeks. }
procedure CalendarDraw;
var
  Ch: array[0..6, 0..20] of Char;
  At: array[0..6, 0..20] of Byte;
  First, Days, D, Row, Col, I, J: Integer;
  Colour: Byte;
begin
  OwlSetText(CalTitle, MonthNames[CalMonth] + ' ' + IntStr(CalYear));
  FillChar(Ch, SizeOf(Ch), ' ');
  FillChar(At, SizeOf(At), $70);        { black on grey }
  for I := 0 to 20 do begin Ch[0, I] := DayNames[I + 1]; At[0, I] := $71; end;
  First := DayOfWeek(CalYear, CalMonth, 1);
  Days := DaysInMonth(CalYear, CalMonth);
  for D := 1 to Days do
  begin
    Row := 1 + (First + D - 1) div 7;
    Col := ((First + D - 1) mod 7) * 3;
    if D < 10 then Ch[Row, Col] := ' ' else Ch[Row, Col] := Chr(Ord('0') + D div 10);
    Ch[Row, Col + 1] := Chr(Ord('0') + D mod 10);
    if Col = 0 then Colour := $74 else Colour := $70;              { Sundays red }
    if (CalYear = TodayYear) and (CalMonth = TodayMonth) and (D = TodayDay) then
      Colour := $2F;                                               { today white on green }
    At[Row, Col] := Colour;
    At[Row, Col + 1] := Colour;
  end;
  OwlBlit(CalCanvas, 0, 0, 21, 7);
  for I := 0 to 6 do
    for J := 0 to 20 do OwlCell(Ord(Ch[I, J]), At[I, J]);
  OwlEnd;
end;

procedure CalendarKey(Cmd: Word);
begin
  if CalWin = 0 then Exit;
  if Cmd = CmToday then begin CalYear := TodayYear; CalMonth := TodayMonth; end
  else if Cmd = CmMonthBack then
  begin
    Dec(CalMonth);
    if CalMonth = 0 then begin CalMonth := 12; Dec(CalYear); end;
  end
  else
  begin
    Inc(CalMonth);
    if CalMonth = 13 then begin CalMonth := 1; Inc(CalYear); end;
  end;
  CalendarDraw;
end;

procedure CalendarShow;
var
  Y, M, D, W: Word;
begin
  if CalWin <> 0 then begin OwlActivate(CalWin); Exit; end;
  GetDate(Y, M, D, W);
  TodayYear := Y; TodayMonth := M; TodayDay := D;
  CalYear := Y; CalMonth := M;
  CalWin := OwlWindow(0, -1, -1, 40, 15, OwlDialog, 'Calendar', CmClose);
  CalTitle := OwlStatic(CalWin, 9, 1, 22, 1, ' ');
  CalCanvas := OwlCanvas(CalWin, 8, 3, 21, 7);
  OwlButtons(CalWin, 3);
  OwlButton(CmMonthBack, 0, '< ~B~ack');
  OwlButton(CmToday, OwlDefault, '~T~oday');
  OwlButton(CmMonthOn, 0, '~O~n >');
  OwlEnd;
  OwlWindowStatus(CalWin, 3);
  OwlKey('', CmMonthBack, KeyNamed, 9, 0);    { PgUp }
  OwlKey('', CmMonthOn, KeyNamed, 10, 0);     { PgDn }
  OwlKey('', CmToday, KeyNamed, 7, 0);        { Home }
  OwlEnd;
  CalendarDraw;
end;

{ ---- the ASCII table ----------------------------------------------------- }

const
  Hex: string[16] = '0123456789ABCDEF';

var
  AsciiCanvas, AsciiInfo: Word;

procedure AsciiShow;
var
  X, Y: Integer;
begin
  if AsciiWin <> 0 then begin OwlActivate(AsciiWin); Exit; end;
  AsciiWin := OwlWindow(0, -1, -1, 39, 21, OwlDialog, 'ASCII table', CmClose);
  AsciiCanvas := OwlCanvas(AsciiWin, 1, 1, 35, 17);
  AsciiInfo := OwlStatic(AsciiWin, 2, 18, 33, 1, 'Click a glyph');
  { 35 by 17: the column numbers across the top, the row numbers down the
    side, and every glyph two columns apart - sent as the character the
    glyph table says it is, so the server finds the glyph again. }
  OwlBlit(AsciiCanvas, 0, 0, 35, 17);
  for Y := 0 to 16 do
    for X := 0 to 34 do
      if Y = 0 then
      begin
        if (X >= 3) and ((X - 3) mod 2 = 0) then OwlCell(Ord(Hex[(X - 3) div 2 + 1]), $74)
        else OwlCell(Ord(' '), $70);
      end
      else if X = 0 then OwlCell(Ord(Hex[Y]), $74)
      else if X = 1 then OwlCell(Ord('0'), $74)
      else if X = 2 then OwlCell(Ord(' '), $74)
      else if (X - 3) mod 2 = 0 then OwlCell(Glyphs[(Y - 1) * 16 + (X - 3) div 2], $1E)
      else OwlCell(Ord(' '), $1E);
  OwlEnd;
end;

{ A character as UTF-8. }
function Utf8(C: Word): string;
begin
  if C < $80 then Utf8 := Chr(C)
  else if C < $800 then Utf8 := Chr($C0 or (C shr 6)) + Chr($80 or (C and $3F))
  else Utf8 := Chr($E0 or (C shr 12)) + Chr($80 or ((C shr 6) and $3F)) + Chr($80 or (C and $3F));
end;

{ A click on the table: which glyph, in every base. }
procedure AsciiPoll;
var
  X, Y, Code: Integer;
begin
  if AsciiWin = 0 then Exit;
  if not OwlClick(AsciiCanvas, X, Y) then Exit;
  if (X < 3) or ((X - 3) mod 2 <> 0) or (Y < 1) or (Y > 16) then Exit;
  Code := (Y - 1) * 16 + (X - 3) div 2;
  OwlSetText(AsciiInfo, 'dec ' + IntStr(Code) + '  hex ' + Hex[Code div 16 + 1] + Hex[Code mod 16 + 1] +
    '  ' + Utf8(Glyphs[Code]));
end;

{ ---- the fifteen puzzle -------------------------------------------------- }

var
  PuzCanvas, PuzMovesText: Word;
  Board: array[0..15] of Byte;          { tiles 1..15, 0 the hole }
  Moves, Seed: Word;

function Hole: Integer;
var
  I: Integer;
begin
  I := 0;
  while (I < 15) and (Board[I] <> 0) do Inc(I);
  Hole := I;
end;

{ A tile next to the hole slides into it. }
function Slide(T: Integer): Boolean;
var
  H, D: Integer;
begin
  Slide := False;
  if (T < 0) or (T > 15) then Exit;
  H := Hole;
  D := T - H;
  if (D = 4) or (D = -4) or (((D = 1) or (D = -1)) and (T div 4 = H div 4)) then
  begin
    Board[H] := Board[T];
    Board[T] := 0;
    Inc(Moves);
    Slide := True;
  end;
end;

{ An arrow: the tile beside the hole on the far side moves into it. }
procedure Arrow(Cmd: Word);
var
  H: Integer;
begin
  H := Hole;
  if (Cmd = CmUp) and (H < 12) then Slide(H + 4)
  else if (Cmd = CmDown) and (H >= 4) then Slide(H - 4)
  else if (Cmd = CmLeft) and (H mod 4 <> 3) then Slide(H + 1)
  else if (Cmd = CmRight) and (H mod 4 <> 0) then Slide(H - 1);
end;

function Solved: Boolean;
var
  I: Integer;
begin
  Solved := True;
  for I := 0 to 14 do if Board[I] <> I + 1 then Solved := False;
end;

{ Solved, then three hundred random slides: every scramble can be undone. }
procedure Scramble;
var
  I: Integer;
begin
  for I := 0 to 14 do Board[I] := I + 1;
  Board[15] := 0;
  Seed := MemW[$40:$6C];                { the BIOS clock }
  for I := 1 to 300 do
  begin
    Seed := Seed * 25173 + 13849;
    Arrow(CmUp + (Hi(Seed) and 3));
  end;
  Moves := 0;
end;

{ A tile is five by two in blue, with a column and a row of the window's
  grey between tiles. }
procedure PuzzleDraw;
var
  X, Y, T, DX, DY: Integer;
begin
  OwlBlit(PuzCanvas, 0, 0, 24, 12);
  for Y := 0 to 11 do
    for X := 0 to 23 do
    begin
      T := Board[(Y div 3) * 4 + X div 6];
      DX := X mod 6;
      DY := Y mod 3;
      if (DX = 5) or (DY = 2) or (T = 0) then OwlCell(Ord(' '), $70)
      else if (DY = 1) and (DX = 2) and (T >= 10) then OwlCell(Ord('1'), $1F)
      else if (DY = 1) and (DX = 3) then OwlCell(Ord('0') + T mod 10, $1F)
      else OwlCell(Ord(' '), $1F);
    end;
  OwlEnd;
  if Solved then OwlSetText(PuzMovesText, 'Solved! Moves: ' + IntStr(Moves))
  else OwlSetText(PuzMovesText, 'Moves: ' + IntStr(Moves));
end;

procedure PuzzleKey(Cmd: Word);
begin
  if PuzWin = 0 then Exit;
  if Cmd = CmScramble then Scramble else Arrow(Cmd);
  PuzzleDraw;
end;

procedure PuzzleShow;
begin
  if PuzWin <> 0 then begin OwlActivate(PuzWin); Exit; end;
  PuzWin := OwlWindow(0, -1, -1, 30, 18, OwlDialog, 'Puzzle', CmClose);
  PuzCanvas := OwlCanvas(PuzWin, 3, 1, 24, 12);
  PuzMovesText := OwlStatic(PuzWin, 3, 13, 24, 1, ' ');
  OwlButtons(PuzWin, 1);
  OwlButton(CmScramble, OwlDefault, '~S~cramble');
  OwlEnd;
  OwlWindowStatus(PuzWin, 4);
  OwlKey('', CmUp, KeyNamed, 11, 0);
  OwlKey('', CmDown, KeyNamed, 12, 0);
  OwlKey('', CmLeft, KeyNamed, 13, 0);
  OwlKey('', CmRight, KeyNamed, 14, 0);
  OwlEnd;
  PuzzleKey(CmScramble);
end;

{ A click on the board: the tile under it, slid if it can go. }
procedure PuzzlePoll;
var
  X, Y: Integer;
begin
  if PuzWin = 0 then Exit;
  if not OwlClick(PuzCanvas, X, Y) then Exit;
  if Slide((Y div 3) * 4 + X div 6) then PuzzleDraw;
end;

{ ---- the desktop's own verbs --------------------------------------------- }

{ The front window is being closed: whoever owns it forgets it. }
procedure WindowClose;
var
  A: Word;
begin
  A := OwlActive;
  if A = 0 then Exit;
  if A = CalcWin then CalcWin := 0;
  if A = CalWin then CalWin := 0;
  if A = AsciiWin then AsciiWin := 0;
  if A = PuzWin then PuzWin := 0;
  if A = Box then Box := 0;
  if A = OpenDialog then begin OpenDialog := 0; OpenPanel := 0; end;
  OwlClose(A);
end;

{ ---- the bars, and the commands ------------------------------------------ }

procedure BuildBars;
begin
  OwlMenuBar(4);
  OwlMenu('~F~ile', 4);
  OwlItem('~N~ew', '', 'An editor window of its own', CmNew);
  OwlItem('~O~pen...', 'F3', 'A file from the disk, in a viewer', CmOpen);
  OwlLine;
  OwlItem('E~x~it', 'Alt+X', 'Leave the program', CmExit);
  OwlMenu('~T~ools', 4);
  OwlItem('~C~alculator', '', 'Add, take, times, share: type or click', CmCalc);
  OwlItem('Ca~l~endar', '', 'A month at a time; PgUp and PgDn page', CmCalendar);
  OwlItem('~A~SCII table', '', 'Every glyph of the font; click one', CmAscii);
  OwlItem('~P~uzzle', '', 'The fifteen puzzle: click a tile, or arrows', CmPuzzle);
  OwlMenu('~W~indow', 8);
  OwlItem('~Z~oom', 'F5', 'The window fills the desktop, or goes back', CmZoom);
  OwlItem('~N~ext', 'F6', 'The front window goes to the back', CmNext);
  OwlItem('~P~revious', 'Shift+F6', 'The window at the back comes to the front', CmPrevious);
  OwlItem('~C~lose', 'Alt+F3', 'Close the front window', CmClose);
  OwlItem('~L~ist...', 'Alt+0', 'Every window by number', CmList);
  OwlLine;
  OwlItem('C~a~scade', '', 'The windows along the diagonal', CmCascade);
  OwlItem('~T~ile', '', 'The windows share the desktop', CmTile);
  OwlMenu('~H~elp', 1);
  OwlItem('~A~bout', '', 'What this program is and what draws it', CmAbout);
  OwlEnd;

  OwlStatus(6);
  OwlKey('~F1~ Help', CmHelp, KeyF, 1, 0);
  OwlKey('~F3~ Open', CmOpen, KeyF, 3, 0);
  OwlKey('~F5~ Zoom', CmZoom, KeyF, 5, 0);
  OwlKey('~F6~ Next', CmNext, KeyF, 6, 0);
  OwlKey('~Alt-F3~ Close', CmClose, KeyF, 3, ModAlt);
  OwlKey('~Alt-X~ Exit', CmExit, KeyChar, Ord('x'), ModAlt);
  OwlEnd;
end;

{ One command. False means leave. }
function Command(Cmd: Word): Boolean;
var
  A: Word;
begin
  Command := True;
  if (Cmd >= CmDigit) and (Cmd <= CmNegate) then CalcKey(Cmd)
  else if (Cmd >= CmMonthBack) and (Cmd <= CmToday) then CalendarKey(Cmd)
  else if (Cmd >= CmScramble) and (Cmd <= CmRight) then PuzzleKey(Cmd)
  else case Cmd of
    CmExit: Command := False;
    CmNew: EditorNew;
    CmOpen: OpenShow;
    CmFileOpen: OpenButton;
    CmOpenCancel: OpenClose;
    CmCalc: CalcShow;
    CmCalendar: CalendarShow;
    CmAscii: AsciiShow;
    CmPuzzle: PuzzleShow;
    CmAbout, CmHelp: About;
    CmDismiss: BoxClose;
    CmNext: OwlCycle;
    CmPrevious: OwlCycleBack;
    CmZoom: begin A := OwlActive; if A <> 0 then OwlZoom(A); end;
    CmClose: WindowClose;
    CmCascade: OwlCascade;
    CmTile: OwlTile;
    CmList: OwlWindowList;
  end;
end;

var
  Pressed, Chosen: Word;
  Running: Boolean;

begin
  if not OwlCheck then Halt(1);
  OwlInit;
  OwlGlyphs(Glyphs);
  BuildBars;
  About;                                { say hello }
  Running := True;
  while Running do
  begin
    OwlWait(Pressed, Chosen);           { a button pressed, a command chosen }
    if Pressed <> 0 then Running := Command(Pressed);
    if Running and (Chosen <> 0) then Running := Command(Chosen);
    AsciiPoll;                          { what is not a command: a click on a canvas }
    PuzzlePoll;
    OpenPoll;                           { and a name entered in the file panel }
  end;
  OwlQuit;
end.
