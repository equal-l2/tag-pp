# tag-pp
情報科学実験I超高性能化課題のためのデータ前処理プログラム  
データ構造は [tag-geotag](https://github.com/equal-l2/tag-geotag) を参照   

    $ tag-pp <サブコマンド>

## サブコマンド一覧  
- tag-pp  
`$ tag-pp tag-pp <tag.csvの場所> <出力先>`  
tagをtag_pp.csv形式に変換  

- geotag-pp  
`$ tag-pp geotag-pp <tag_pp.csvの場所> <geotag.csvの場所> <出力先>`  
geotagをgeotag_pp.csv形式に変換  

- gen-test  
`$ tag-pp gen-test <tag.csv> <geotag.csv> <行数>`  
元ファイルからうまいこと切り出してテスト用の小さなtag_pp.csvとgeotag_pp.csvを作る
