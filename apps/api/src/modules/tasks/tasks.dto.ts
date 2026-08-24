import { IsArray, IsIn, IsOptional, IsString, MaxLength, MinLength } from "class-validator";
import type { Priority, PublishTaskInput, ClaimTaskInput } from "@buling/shared";

export class PublishTaskDto implements PublishTaskInput {
  @IsString() @MinLength(1) @MaxLength(200)
  title!: string;

  @IsOptional() @IsString() @MaxLength(4000)
  description?: string;

  @IsArray() @IsString({ each: true })
  requiredSkills!: string[];

  @IsOptional() @IsIn(["high", "mid", "low"])
  priority?: Priority;

  @IsOptional() @IsString()
  due?: string;

  @IsIn(["auto", "manual"])
  assignMode!: "auto" | "manual";

  @IsOptional() @IsString()
  assigneeAgentId?: string;
}

export class ClaimTaskDto implements ClaimTaskInput {
  @IsString()
  agentId!: string;
}
