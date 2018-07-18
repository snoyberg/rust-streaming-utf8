#!/usr/bin/env stack
-- stack --resolver lts-12.0 script
import Conduit
import Data.HashMap.Strict (HashMap)
import qualified Data.HashMap.Strict as HashMap

mapper :: HashMap Char Char
mapper = HashMap.fromList $ zip
  "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
  "ДВСDЁҒGНІЈКLМПОРQЯЅТЦЏШХЧZавсdёfgніјкlмпорqгѕтцѵшхчz"

convertChar :: Char -> Char
convertChar c = HashMap.lookupDefault c c mapper

main :: IO ()
main = runConduit
     $ stdinC
    .| decodeUtf8C
    .| omapCE convertChar
    .| encodeUtf8C
    .| stdoutC
