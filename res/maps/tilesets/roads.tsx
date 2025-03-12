<?xml version="1.0" encoding="UTF-8"?>
<tileset version="1.10" tiledversion="1.11.0" name="roads" tilewidth="16" tileheight="16" tilecount="64" columns="8">
 <image source="../../textures/tiles/roads.png" width="128" height="128"/>
 <tile id="12">
  <properties>
   <property name="blocking" type="bool" value="true"/>
  </properties>
 </tile>
 <tile id="32">
  <properties>
   <property name="exits" value="horizontal"/>
  </properties>
 </tile>
 <tile id="33">
  <properties>
   <property name="exits" value="horizontal"/>
  </properties>
 </tile>
 <tile id="34">
  <properties>
   <property name="exits" value="left"/>
  </properties>
 </tile>
 <tile id="35">
  <properties>
   <property name="exits" value="right"/>
  </properties>
 </tile>
 <tile id="36">
  <properties>
   <property name="exits" value="not_right"/>
  </properties>
 </tile>
 <tile id="40">
  <properties>
   <property name="exits" value="vertical"/>
  </properties>
 </tile>
 <tile id="41">
  <properties>
   <property name="exits" value="vertical"/>
  </properties>
 </tile>
 <tile id="42">
  <properties>
   <property name="exits" value="up"/>
  </properties>
 </tile>
 <tile id="43">
  <properties>
   <property name="exits" value="down"/>
  </properties>
 </tile>
 <tile id="48">
  <properties>
   <property name="exits" value="corner_ul"/>
  </properties>
 </tile>
 <tile id="49">
  <properties>
   <property name="exits" value="corner_ur"/>
  </properties>
 </tile>
 <tile id="50">
  <properties>
   <property name="exits" value="not_up"/>
  </properties>
 </tile>
 <tile id="56">
  <properties>
   <property name="exits" value="corner_dl"/>
  </properties>
 </tile>
 <tile id="57">
  <properties>
   <property name="exits" value="corner_dr"/>
  </properties>
 </tile>
 <tile id="58">
  <properties>
   <property name="exits" value="not_down"/>
  </properties>
 </tile>
 <tile id="59">
  <properties>
   <property name="exits" value="not_left"/>
  </properties>
 </tile>
 <wangsets>
  <wangset name="Roads" type="edge" tile="-1">
   <wangcolor name="Dark roads" color="#ff0000" tile="-1" probability="1"/>
   <wangtile tileid="33" wangid="0,0,1,0,0,0,1,0"/>
   <wangtile tileid="34" wangid="0,0,0,0,0,0,1,0"/>
   <wangtile tileid="35" wangid="0,0,1,0,0,0,0,0"/>
   <wangtile tileid="36" wangid="1,0,0,0,1,0,1,0"/>
   <wangtile tileid="41" wangid="1,0,0,0,1,0,0,0"/>
   <wangtile tileid="42" wangid="1,0,0,0,0,0,0,0"/>
   <wangtile tileid="43" wangid="0,0,0,0,1,0,0,0"/>
   <wangtile tileid="48" wangid="0,0,1,0,1,0,0,0"/>
   <wangtile tileid="49" wangid="0,0,0,0,1,0,1,0"/>
   <wangtile tileid="50" wangid="0,0,1,0,1,0,1,0"/>
   <wangtile tileid="51" wangid="1,0,1,0,1,0,1,0"/>
   <wangtile tileid="56" wangid="1,0,1,0,0,0,0,0"/>
   <wangtile tileid="57" wangid="1,0,0,0,0,0,1,0"/>
   <wangtile tileid="58" wangid="1,0,1,0,0,0,1,0"/>
   <wangtile tileid="59" wangid="1,0,1,0,1,0,0,0"/>
  </wangset>
 </wangsets>
</tileset>
