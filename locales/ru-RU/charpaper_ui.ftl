# Русский перевод. Для проверки положите эту папку рядом с charpaper.exe:
# `locales/ru-RU/charpaper_ui.ftl`.

language-name = Русский

## Panel

tab-scene = Сцена
tab-graphics = Графика
tab-sound = Звук
tab-system = Система
quit = Выход

## Scene tab

section-suite = ПЕРСОНАЖ
label-suite = Персонаж
section-outfit = ОДЕЖДА
section-parts = Части
section-animation = АНИМАЦИЯ
label-animation = Анимация
section-expression = МИМИКА
label-expression = Мимика
section-gaze = ВЗГЛЯД
label-follow-cursor = За курсором
expression-neutral = Нейтрально
section-camera = КАМЕРА
label-camera = Камера
camera-orbit = Орбита
section-environment = ОКРУЖЕНИЕ
label-environment = Окружение
label-brightness = Яркость
label-shadows = Тени

## Graphics tab

section-quality = КАЧЕСТВО
label-render-scale = Масштаб
label-anti-aliasing = Сглаживание
label-depth-of-field = Глубина резкости
label-shadow-quality = Качество теней
section-fog = ТУМАН
label-fog = Туман
label-fog-quality = Качество
label-fog-dithering = Дизеринг
label-fog-resolution = Разрешение
section-post-processing = ПОСТОБРАБОТКА
section-image = ИЗОБРАЖЕНИЕ
label-tonemapping = Тонмаппинг
label-exposure = Экспозиция
label-bloom = Свечение
label-bloom-threshold = Порог свечения
label-chromatic-aberration = Аберрация
section-color = ЦВЕТ
label-warmth = Теплота
label-tint = Оттенок
label-saturation = Насыщенность
label-contrast = Контраст
label-lut = LUT
label-lut-strength = Сила LUT
section-film = ПЛЁНКА
label-vignette = Виньетка
label-vignette-size = Размер виньетки
label-grain = Зерно
label-grain-size = Размер зерна
restore-look = Сбросить настройки
value-blur = Размытие
value-bokeh = Боке
value-very-low = Очень низкое
value-low = Низкое
value-medium = Среднее
value-high = Высокое
value-ultra = Ультра
value-full = Полное
value-half = Половина

## Sound tab

section-sound = ЗВУК
label-master-volume = Громкость
label-music-volume = Музыка
label-ambience-volume = Атмосфера
label-character-volume = Персонаж

## System tab

section-app = ПРИЛОЖЕНИЕ
label-mode = Показывать как
mode-wallpaper = Обои
mode-window = Окно
label-start-at-login = Запуск при входе
section-display = ЭКРАН
stage-name = Сцена { $letter }
monitor-missing = не подключён
label-layout = Раскладка
layout-mirror = Зеркало
layout-span = Панорама
section-frame-rate = ЧАСТОТА КАДРОВ
label-fps-limit = Лимит FPS
label-monitor = Монитор производительности
value-max = Макс.
section-pause = ПАУЗА, КОГДА
label-pause-fullscreen = Полный экран
label-pause-covered = Стол закрыт
label-pause-battery = От батареи
section-interface = ИНТЕРФЕЙС
label-language = Язык
language-system = Системный

## Performance monitor, shown at the top of the screen. Units after numbers.

monitor-title = ПРОИЗВОДИТЕЛЬНОСТЬ
monitor-ms = { $value } мс
monitor-mb = { $value } МБ
monitor-frame-rate = Частота кадров
monitor-fps = { $value } к/с
monitor-frame-time = { $time } на кадр
monitor-cpu = Процессор
# How many of the computer's cores CharPaper keeps busy, on average.
monitor-cores = занято ядер: { $busy } из { $total }
monitor-main-thread = Основной поток
monitor-render-thread = Поток отрисовки
monitor-recording = { $time } на запись команд GPU
monitor-stage-assets = Ресурсы
monitor-stage-specialize = Варианты шейдеров
monitor-stage-views = Виды, экран
monitor-stage-views-note = здесь ждёт, если GPU или вертикальная синхронизация медленнее
monitor-stage-queue = Сортировка отрисовки
monitor-stage-prepare = Буферы GPU
monitor-stage-render = Команды
monitor-stage-cleanup = Очистка
# The memory section. "Memory" is all the RAM the process touches, as Task Manager counts it.
monitor-memory = Память
# How much a kind of memory grew over the last minute, with its sign.
monitor-growth = { $mb } МБ в минуту
# Memory the process asked Windows for, touched or not.
monitor-committed = Выделено
# What the app's own code holds, and in how many separate pieces.
monitor-heap = Куча
monitor-heap-blocks = блоков: { $count }
monitor-gpu-memory = Память GPU
monitor-gpu-reserved = { $mb } МБ зарезервировано
monitor-gpu-objects = Объекты GPU
monitor-gpu-object-details = буферы { $buffers }, текстуры { $textures }, виды { $views }, группы привязок { $groups }, сэмплеры { $samplers }, конвейеры { $pipelines }, шейдеры { $shaders }
monitor-entities = Сущности
monitor-render-entities = в мире отрисовки: { $count }
monitor-assets = Ресурсы
monitor-asset-details = изображения { $images }, меши { $meshes }, материалы { $materials }, клипы { $clips }, glTF { $gltfs }
monitor-graphics = Видеокарта
monitor-discrete = дискретная
monitor-integrated = встроенная
monitor-software = без GPU, рисует процессор
monitor-other-gpu = другая
monitor-gpu = GPU
monitor-busy = { $percent } времени
monitor-waiting = идёт замер…
monitor-unsupported = эта видеокарта не умеет замерять время
monitor-light-shadows = Тени ламп
# Of the shadow maps there are, how many were drawn again this frame; the rest show what
# they showed before, since nothing in them changed.
monitor-shadow-maps = карт теней перерисовано: { $drawn } из { $count }
monitor-prepass = Предпроход, тени солнца
monitor-main-pass = Основной проход
monitor-fog = Туман
monitor-effects = Эффекты
monitor-display = Экран, панель
monitor-scene = Сцена
monitor-lights = источников света: { $count }
monitor-light-details = с тенями: { $shadowed }, в тумане: { $fog }, объёмов тумана: { $boxes }
monitor-meshes = Меши
# How many times a frame meshes are drawn: by the cameras, and into the lights' shadow maps.
monitor-mesh-details = отрисовок камерами: { $drawn }, в карты теней: { $shadows }

## Values shown in controls

value-on = Вкл
value-off = Выкл

## Подсказка внизу панели

help-panel = Наведите на настройку, чтобы узнать, что она делает. Колесо мыши над значением меняет его.
help-suite = Какого персонажа показывать, из папок в characters.
help-animation = Что делает персонаж.
help-expression = Выражение лица персонажа.
help-follow-cursor = Персонаж поворачивает голову и смотрит вслед за указателем мыши.
help-camera = Откуда видна сцена. Орбиту двигают мышью над сценой.
help-environment = Окружение со своим светом и небом.
help-brightness = Насколько ярко небо окружения и свет от него.
help-shadows = Отбрасывает ли свет окружения тени. Без теней быстрее.
help-tonemapping = Как яркие цвета укладываются в возможности экрана. Каждый способ задаёт своё настроение.
help-exposure = Делает всю картинку светлее или темнее, как экспозиция в фотоаппарате.
help-bloom = Свечение вокруг самых ярких мест картинки.
help-bloom-threshold = Насколько ярким должно быть место, чтобы светиться: 1.0 — белый, ниже — светятся и более тусклые огни.
help-chromatic-aberration = Цветная кайма к краям кадра, как у простого объектива.
help-vignette = Затемняет углы картинки.
help-vignette-size = Как далеко к центру заходит затемнение.
help-grain = Мелкий шум поверх картинки, как у плёнки.
help-grain-size = Насколько крупное зерно.
help-warmth = Сдвигает цвета к оранжевому или к синему.
help-tint = Сдвигает цвета к пурпурному или к зелёному.
help-saturation = Насколько сочные цвета. При 0% картинка чёрно-белая.
help-contrast = Насколько сильно разведены тёмное и светлое.
help-lut = Готовая цветокоррекция из папки luts персонажа.
help-lut-strength = Насколько сильно применять цветокоррекцию.
help-fps-limit = Сколько кадров в секунду рисовать. Меньше кадров — меньше расход энергии.
help-monitor = Показывает вверху экрана, сколько каждый этап отрисовки кадра стоит процессору и видеокарте.
help-render-scale = Рисует сцену в меньшем разрешении ради экономии энергии. Панель остаётся чёткой.
help-anti-aliasing = Сглаживает зубчатые края. TAA сглаживает лучше всех, но может оставлять след за движением.
help-depth-of-field = Размывает то, что не в фокусе, как объектив. Боке выглядит лучше, размытие быстрее.
help-shadow-quality = Насколько чёткие тени, на всех сценах. Низкое быстрее всего, но края жёсткие. Высокое занимает намного больше видеопамяти.
help-fog = Туман и лучи света, если они есть в окружении.
help-fog-quality = Насколько подробно рисуется туман. Чем выше, тем дороже.
help-fog-dithering = Убирает полосы в тумане мелким шумом.
help-fog-resolution = В половинном разрешении туман рисуется в четыре раза быстрее. Он мягкий, поэтому разница почти не видна.
help-pause-fullscreen = Останавливает картинку и звук, пока приложение, например игра, открыто на весь экран.
help-pause-covered = Останавливает картинку и звук, пока окна закрывают весь рабочий стол.
help-pause-battery = Останавливает картинку и звук, пока компьютер работает от батареи.
help-mode = «Обои» ставят сцену за значки рабочего стола, «Окно» показывает её в обычном окне. Для переключения CharPaper перезапустится.
help-start-at-login = Запускает CharPaper при каждом входе в систему.
help-layout = Для сцены этого монитора. «Зеркало» показывает всю сцену на каждом её мониторе. «Панорама» растягивает один широкий вид на них, будто это один экран.
help-master-volume = Общая громкость всех звуков CharPaper.
help-music-volume = Музыка окружения.
help-ambience-volume = Фоновые звуки окружения, например дождь или ветер.
help-character-volume = Звуки анимаций персонажа, например шаги.
help-language = Язык панели. «Системный» следует языку системы.
